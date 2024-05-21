use std::{
    cell::{RefCell, UnsafeCell},
    sync::{atomic::{AtomicU64, AtomicUsize}, Arc}, u8,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net,
    sync::mpsc,
    time::timeout,
};
use tracing::debug;

use crate::{
    common::TimeoutOptions, error::RpcError, message::{ReqType, RespType}, packet::{
        Decode, Encode, Packet, PacketTask, ReqHeader, RespHeader, REQ_HEADER_SIZE
    }
};

/// TODO: combine RpcClientConnectionInner and RpcClientConnection
struct RpcClientConnectionInner<T>
    where T: Packet + Clone + Send + Sync + 'static
{
    /// The TCP stream for the connection.
    stream: UnsafeCell<net::TcpStream>,
    /// Options for the timeout of the connection
    timeout_options: TimeoutOptions,
    /// Stream auto increment sequence number, used to mark the request and response
    seq: AtomicU64,
    /// send packet task
    packet_task: UnsafeCell<PacketTask<T>>
}

impl<P> RpcClientConnectionInner<P>
where P: Packet + Clone + Send + Sync + 'static
{
    pub fn new(stream: net::TcpStream, timeout_options: TimeoutOptions) -> Self {
        Self {
            stream: UnsafeCell::new(stream),
            timeout_options: timeout_options.clone(),
            seq: AtomicU64::new(0),
            packet_task: UnsafeCell::new(PacketTask::new(timeout_options.clone().idle_timeout.as_secs())),
        }
    }

    /// Recv request header from the stream
    pub async fn recv_header(&self) -> Result<RespHeader, RpcError<String>> {
        let req_header_buffer = self.recv_len(REQ_HEADER_SIZE).await?;
        let req_header = RespHeader::decode(&req_header_buffer)?;
        debug!("Received request header: {:?}", req_header);

        Ok(req_header)
    }

    /// Receive request body from the server.
    pub async fn recv_len(&self, len: u64) -> Result<Vec<u8>, RpcError<String>> {
        let mut req_buffer = vec![0u8; len as usize];
        let reader = self.get_stream_mut();
        match timeout(
            self.timeout_options.read_timeout,
            reader.read_exact(&mut req_buffer),
        )
        .await
        {
            Ok(result) => match result {
                Ok(_) => {
                    debug!("Received request body: {:?}", req_buffer);
                    return Ok(req_buffer);
                }
                Err(err) => {
                    debug!("Failed to receive request header: {:?}", err);
                    return Err(RpcError::InternalError(err.to_string()));
                }
            },
            Err(_) => {
                debug!("Timeout to receive request header");
                return Err(RpcError::InternalError(
                    "Timeout to receive request header".to_string(),
                ));
            }
        }
    }

    /// Send a request to the server.
    pub async fn send_data(&self, data: &[u8]) -> Result<(), RpcError<String>> {
        let writer = self.get_stream_mut();
        match timeout(self.timeout_options.write_timeout, writer.write_all(data)).await {
            Ok(result) => match result {
                Ok(_) => {
                    debug!("Sent data: {:?}", data);
                    return Ok(());
                }
                Err(err) => {
                    debug!("Failed to send data: {:?}", err);
                    return Err(RpcError::InternalError(err.to_string()));
                }
            },
            Err(_) => {
                debug!("Timeout to send data");
                return Err(RpcError::InternalError("Timeout to send data".to_string()));
            }
        }
    }

    /// Get the next sequence number.
    pub fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// Send loop for the client.
    pub async fn send_loop(&self, request_channel_rx: &mut mpsc::Receiver<P>) {
        // Tickers to keep alive the connection
        let mut tickers = tokio::time::interval(self.timeout_options.idle_timeout / 3);
        loop {
            tokio::select! {
                _ = tickers.tick() => {
                    // Send keep alive message
                    let keep_alive_msg = ReqHeader {
                        seq: self.next_seq(),
                        op: ReqType::KeepAliveRequest.to_u8(),
                        len: 0,
                    }.encode();

                    if let Ok(_) =  self.send_data(&keep_alive_msg).await {
                        debug!("Sent keep alive message");
                        // Set to packet task
                    } else {
                        debug!("Failed to send keep alive message");
                        break;
                    }
                }
                req_result = request_channel_rx.recv() => {
                    match req_result {
                        Some(req) => {
                            if let Ok(req_buffer) = req.serialize() {
                                let req_header = ReqHeader {
                                    seq: req.seq(),
                                    op: req.op(),
                                    len: req_buffer.len() as u64,
                                }.encode();

                                if let Ok(_) = self.send_data(&req_header).await {
                                    debug!("Sent request header: {:?}", req_header);
                                    // Set to packet task
                                    self.get_packet_task_mut().add_task(req.clone()).await;
                                } else {
                                    debug!("Failed to send request header: {:?}", req.seq());
                                    break;
                                }

                                debug!("Sent request: {:?}", req_buffer);

                            } else {
                                debug!("Failed to serialize request: {:?}", req.seq());
                                break;
                            }
                        }
                        None => {
                            // The request channel is closed and no remaining requests
                            debug!("Request channel closed.");
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Receive loop for the client.
    pub async fn recv_loop(&self, response_channel_tx: mpsc::Sender<P>) {
        loop {
            debug!("Waiting for response...");
            let resp_header = self.recv_header().await;
            match resp_header {
                Ok(header) => {
                    let header_seq = header.seq;
                    if let Ok(resp_type) = RespType::from_u8(header.op) {
                        match resp_type {
                            RespType::KeepAliveResponse => {
                                debug!("Received keep alive response.");

                                // Try to get and check the packet
                                let packet_task = self.get_packet_task_mut();
                                let _ = packet_task.get_task(header_seq);

                                continue;
                            }
                            _ => {
                                debug!("Received response header: {:?}", header);
                                let resp_buffer = match self.recv_len(header.len).await {
                                    Ok(buffer) => buffer,
                                    Err(err) => {
                                        debug!("Failed to receive request body: {:?}", err);
                                        break;
                                    }
                                };

                                // Send data to the response channel
                                // We need to check channel data's type and send it to the corresponding channel
                                // 1. Use a fixed number in resp_buffer and check the type?
                                // 2. Or merge the header and buffer and send it to the channel?
                                // header buffer with body buffer

                                // Take the packet task and recv the response
                                let packet_task: &mut PacketTask<P> = self.get_packet_task_mut();
                                if let Some(body) = packet_task.get_task(header_seq).await {
                                    // Try to fill the packet with the response
                                    body.deserialize(resp_buffer.as_slice()).unwrap();
                                    if let Ok(_) = response_channel_tx.send(body.clone()).await {
                                        debug!("Sent response body");
                                    } else {
                                        debug!("Failed to send response body");
                                        break;
                                    }
                                } else {
                                    debug!("Failed to get packet task");
                                    break;
                                }
                            }
                        }
                    } else {
                        debug!("Invalid response type: {:?}", header.op);
                        break;
                    }
                }
                Err(err) => {
                    debug!("Failed to receive request header: {:?}", err);
                    break;
                }
            }
        }
    }

    /// Get stream with mutable reference
    #[inline(always)]
    fn get_stream_mut(&self) -> &mut net::TcpStream {
        // Current implementation is safe because the stream is only accessed by one thread
        unsafe { std::mem::transmute(self.stream.get()) }
    }

    /// Get stream with immutable reference
    #[inline(always)]
    fn get_stream(&self) -> &net::TcpStream {
        unsafe { std::mem::transmute(self.stream.get()) }
    }

    /// Get packet task with mutable reference
    #[inline(always)]
    fn get_packet_task_mut(&self) -> &mut PacketTask<P> {
        // Current implementation is safe because the packet task is only accessed by one thread
        unsafe { std::mem::transmute(self.packet_task.get()) }
    }

    /// Get packet task with immutable reference
    #[inline(always)]
    fn get_packet_task(&self) -> &PacketTask<P> {
        unsafe { std::mem::transmute(self.packet_task.get()) }
    }
}

unsafe impl<P> Send for RpcClientConnectionInner<P>
    where P: Packet + Clone + Send + Sync + 'static
{}

unsafe impl<P> Sync for RpcClientConnectionInner<P>
    where P: Packet + Clone + Send + Sync + 'static
{}

/// The RPC client definition.
pub struct RpcClient<P>
where P: Packet + Clone + Send + Sync + 'static,
{
    /// Request channel for buffer
    request_channel_tx: mpsc::Sender<P>,
    /// Response channel for buffer
    response_channel_rx: RefCell<mpsc::Receiver<P>>,
}

impl<P> RpcClient<P>
    where P: Packet + Clone + Send + Sync + 'static
{
    /// Create a new RPC client.
    pub async fn new(addr: &str, timeout_options: TimeoutOptions) -> Self {
        let stream = net::TcpStream::connect(addr)
            .await
            .expect("Failed to connect to the server");
        // TODO: use bounded channel
        let (request_channel_tx, mut request_channel_rx) = mpsc::channel::<P>(1000);
        let (response_channel_tx, response_channel_rx) = mpsc::channel::<P>(1000);
        let inner_connection = Arc::new(RpcClientConnectionInner::new(
            stream,
            timeout_options.clone(),
        ));

        // Create a keepalive send loop
        let inner_connection_clone = inner_connection.clone();
        tokio::spawn(async move {
            inner_connection_clone
                .send_loop(&mut request_channel_rx)
                .await;
        });

        // Create a keepalive receive loop
        let inner_connection_clone = inner_connection.clone();
        tokio::spawn(async move {
            inner_connection_clone.recv_loop(response_channel_tx).await;
        });

        Self {
            request_channel_tx,
            response_channel_rx: RefCell::new(response_channel_rx),
        }
    }

    /// Send a request to the server.
    /// Try to send data to channel, if the channel is full, return an error.
    /// Contains the request header and body.
    pub async fn send_request(&self, req: P) -> Result<(), RpcError<String>> {
        self.request_channel_tx
            .send(req)
            .await
            .map_err(|e| RpcError::InternalError(e.to_string()))
    }

    /// Receive a response from the server.
    pub async fn recv_response(&self) -> Result<P, RpcError<String>> {
        self.response_channel_rx
            .borrow_mut()
            .recv()
            .await
            .ok_or(RpcError::InternalError(
                "Failed to receive response".to_string(),
            ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::TimeoutOptions;
    use std::time::Duration;

    #[derive(Debug, Clone)]
    pub struct TestPacket {
        pub seq: u64,
        pub op: u8,
        pub status: u8,
    }

    impl Packet for TestPacket {
        fn seq(&self) -> u64 {
            self.seq
        }

        fn set_seq(&mut self, seq: u64) {
            self.seq = seq;
        }

        fn op(&self) -> u8 {
            self.op
        }

        fn set_op(&mut self, op: u8) {
            self.op = op;
        }

        fn serialize(&self) -> Result<Vec<u8>, RpcError<String>> {
            Ok(vec![0u8; 0])
        }

        fn deserialize(&mut self, _data: &[u8]) -> Result<(), RpcError<String>> {
            Ok(())
        }

        fn status(&self) -> u8 {
            self.status
        }

        fn set_status(&mut self, status: u8) {
            self.status = status;
        }
    }

    #[tokio::test]
    async fn test_rpc_client() {
        // Set the tracing log level to debug
        tracing::subscriber::set_global_default(
            tracing_subscriber::FmtSubscriber::builder()
                .with_max_level(tracing::Level::DEBUG)
                .finish(),
        )
        .expect("Failed to set tracing subscriber");

        let addr = "127.0.0.1:2789";
        let timeout_options = TimeoutOptions {
            read_timeout: Duration::from_secs(20),
            write_timeout: Duration::from_secs(20),
            idle_timeout: Duration::from_secs(20),
        };

        // Create a fake server, will directly return the request
        tokio::spawn(async move {
            let listener = net::TcpListener::bind(addr).await.unwrap();
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let (mut reader, mut writer) = stream.into_split();
                let mut buffer = vec![0u8; 1024];
                let size = reader.read(&mut buffer).await.unwrap();
                debug!("Received request: {:?}", &buffer[..size]);
                // Create a response header
                let resp_header = RespHeader {
                    seq: 0,
                    op: RespType::KeepAliveResponse.to_u8(),
                    len: 0,
                };
                let resp_header = resp_header.encode();
                writer.write_all(&resp_header).await.unwrap();
                debug!("Sent response: {:?}", resp_header);
            }
        });

        // Wait for the server to start
        tokio::time::sleep(Duration::from_secs(2)).await;

        let rpc_client = RpcClient::<TestPacket>::new(addr, timeout_options).await;

        // Wait for the server to start
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Current implementation will send keep alive message every 20/3 seconds
        let resp = rpc_client.recv_response().await;
        assert!(resp.is_err());
    }
}
