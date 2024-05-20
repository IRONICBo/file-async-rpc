use std::{
    borrow::BorrowMut, cell::{RefCell, UnsafeCell}, f32::consts::E, sync::{atomic::AtomicUsize, Arc}
};

use async_trait::async_trait;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net,
    sync::mpsc,
    time::timeout,
};
use tracing::debug;

use crate::{
    common::TimeoutOptions,
    error::RpcError,
    message::{
        decode_resp_header, Encode, FileBlockResponse, ReqHeader, ReqType,
        RespHeader, RespType, REQ_HEADER_SIZE,
    },
    workerpool::WorkerPool,
};

#[derive(Clone)]
struct RpcSenderWorkerFactory {
    worker_pool: WorkerPool,
}

impl RpcSenderWorkerFactory {
    pub fn new(max_workers: usize, max_jobs: usize) -> Self {
        Self {
            worker_pool: WorkerPool::new(max_workers, max_jobs),
        }
    }
}

#[async_trait]
pub trait RpcClientConnectionHandler {
    async fn dispatch(&self, req_header: RespHeader, req_buffer: Vec<u8>);
}

/// The handler for the RPC file block request.
pub struct FileBlockHandler {
    response: FileBlockResponse,
    done_tx: mpsc::Sender<Vec<u8>>,
}

impl FileBlockHandler {
    pub fn new(response: FileBlockResponse, done_tx: mpsc::Sender<Vec<u8>>) -> Self {
        Self { response, done_tx }
    }
}

/// TODO: combine RpcClientConnectionInner and RpcClientConnection
struct RpcClientConnectionInner<T>
where
    T: RpcClientConnectionHandler + Send + Sync + 'static,
{
    /// The TCP stream for the connection.
    stream: UnsafeCell<net::TcpStream>,
    /// Options for the timeout of the connection
    timeout_options: TimeoutOptions,
    /// The handler for the connection
    dispatch_handler: T,
    /// Atomic keep alive sequence
    keep_alive_seq: AtomicUsize,
}

impl<T> RpcClientConnectionInner<T>
where
    T: RpcClientConnectionHandler + Send + Sync + 'static,
{
    pub fn new(
        stream: net::TcpStream,
        timeout_options: TimeoutOptions,
        dispatch_handler: T,
    ) -> Self {
        Self {
            stream: UnsafeCell::new(stream),
            timeout_options,
            dispatch_handler,
            keep_alive_seq: AtomicUsize::new(0),
        }
    }

    /// Recv request header from the stream
    pub async fn recv_header(&self) -> Result<RespHeader, RpcError<String>> {
        let req_header_buffer = self.recv_len(REQ_HEADER_SIZE).await?;
        let req_header = decode_resp_header(&req_header_buffer)?;
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

    /// Send loop for the client.
    pub async fn send_loop(&self, request_channel_rx: &mut mpsc::Receiver<Vec<u8>>) {
        // Tickers to keep alive the connection
        let mut tickers = tokio::time::interval(self.timeout_options.idle_timeout / 3);
        loop {
            tokio::select! {
                _ = tickers.tick() => {
                    // Send keep alive message
                    let keep_alive_msg = ReqHeader {
                        seq: self.keep_alive_seq.load(std::sync::atomic::Ordering::Relaxed) as u64,
                        req_type: ReqType::KeepAliveRequest,
                        len: 0,
                    }.encode();

                    if let Ok(_) =  self.send_data(&keep_alive_msg).await {
                        debug!("Sent keep alive message");
                    } else {
                        debug!("Failed to send keep alive message");
                        break;
                    }
                }
                req_result = request_channel_rx.recv() => {
                    match req_result {
                        Some(req) => {
                            if let Ok(_) = self.send_data(&req).await {
                                debug!("Sent request: {:?}", req);
                            } else {
                                debug!("Failed to send request: {:?}", req);
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
    pub async fn recv_loop(&self, response_channel_tx: mpsc::Sender<Vec<u8>>) {
        loop {
            let resp_header = self.recv_header().await;
            match resp_header {
                Ok(header) => {
                    match header.resp_type {
                        RespType::KeepAliveResponse => {
                            debug!("Received keep alive response.");
                            let current_keep_alive_seq = self
                                .keep_alive_seq
                                .load(std::sync::atomic::Ordering::Relaxed);
                            if header.seq != current_keep_alive_seq as u64 {
                                debug!(
                                    "Keep alive sequence mismatch: {} != {}",
                                    header.seq, current_keep_alive_seq
                                );
                            }

                            // Received keep alive response, increment the keep alive sequence
                            self.keep_alive_seq
                                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                            continue;
                        }
                        _ => {
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
                            if let Ok(_) = response_channel_tx.send(header.encode()).await {
                                debug!("Sent response header: {:?}", header);
                            } else {
                                debug!("Failed to send response header: {:?}", header);
                                break;
                            }
                            if let Ok(_) = response_channel_tx.send(resp_buffer).await {
                                debug!("Sent response body");
                            } else {
                                debug!("Failed to send response body");
                                break;
                            }
                        }
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
}

unsafe impl<T> Send for RpcClientConnectionInner<T> where
    T: RpcClientConnectionHandler + Send + Sync + 'static
{
}

unsafe impl<T> Sync for RpcClientConnectionInner<T> where
    T: RpcClientConnectionHandler + Send + Sync + 'static
{
}

/// The RPC client definition.
pub struct RpcClient<T>
where
    T: RpcClientConnectionHandler + Send + Sync + 'static,
{
    /// Options for the timeout of the server connection
    timeout_options: TimeoutOptions,
    /// Request channel for buffer
    request_channel_tx: mpsc::Sender<Vec<u8>>,
    /// Response channel for buffer
    response_channel_rx: RefCell<mpsc::Receiver<Vec<u8>>>,
    /// Dispatch handler
    dispatch_handler: T,
}

impl<T> RpcClient<T>
where
    T: RpcClientConnectionHandler + Send + Sync + 'static,
{
    /// Create a new RPC client.
    pub async fn new(addr: &str, timeout_options: TimeoutOptions, dispatch_handler: T) -> Self {
        let stream = net::TcpStream::connect(addr)
            .await
            .expect("Failed to connect to the server");
        // TODO: use bounded channel
        let (request_channel_tx, mut request_channel_rx) = mpsc::channel::<Vec<u8>>(1000);
        let (response_channel_tx, response_channel_rx) = mpsc::channel::<Vec<u8>>(1000);
        let inner_connection = Arc::new(RpcClientConnectionInner::new(
            stream,
            timeout_options.clone(),
            dispatch_handler,
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
            timeout_options: timeout_options.clone(),
            request_channel_tx,
            response_channel_rx: RefCell::new(response_channel_rx),
        }
    }

    /// Send a request to the server.
    /// Try to send data to channel, if the channel is full, return an error.
    /// Contains the request header and body.
    pub async fn send_request(&self, req: Vec<u8>) -> Result<(), RpcError<String>> {
        self.request_channel_tx
            .send(req)
            .await
            .map_err(|e| RpcError::InternalError(e.to_string()))
    }

    /// Receive a response from the server.
    pub async fn recv_response(&self) -> Result<Vec<u8>, RpcError<String>> {
        self.response_channel_rx
            .borrow_mut()
            .recv()
            .await
            .ok_or(RpcError::InternalError("Failed to receive response".to_string()))
    }
}
