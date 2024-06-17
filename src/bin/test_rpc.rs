use std::{sync::Arc, time::Duration};

use bytes::BytesMut;
use file_async_rpc::{client::RpcClient, common::ServerTimeoutOptions, common::ClientTimeoutOptions, connect_timeout, error::RpcError, message::ReqType, packet::{Encode, Packet, PacketStatus, ReqHeader, RespHeader}, server::{FileBlockRpcServerHandler, RpcServer, RpcServerConnectionHandler}, workerpool::{Job, WorkerPool}};
use tokio::{net::TcpStream, sync::mpsc, time::Instant};
use tonic::async_trait;
use tracing::{debug, error, info};

// 4MB
const MAX_PACKET_SIZE: usize = 4 * 1024 * 1024;
const MAX_PACKET_NUM: usize = 5000;

/// Check if the port is in use
async fn is_port_in_use(addr: &str) -> bool {
    if let Ok(stream) = TcpStream::connect(addr).await {
        // Port is in use
        drop(stream);
        true
    } else {
        // Port is not in use
        false
    }
}

#[derive(Debug, Clone)]
pub struct TestPacket {
    pub seq: u64,
    pub op: u8,
    pub status: PacketStatus,
    pub buffer: BytesMut,
}

impl TestPacket {
    pub fn new(op: u8) -> Self {
        Self { seq:0, op, status:PacketStatus::Pending, buffer: BytesMut::with_capacity(MAX_PACKET_SIZE)}
    }
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

    fn set_req_data(&mut self, data: &[u8]) -> Result<(), RpcError<String>> {
        // Try to set the request data
        debug!("Setting request data");

        Ok(())
    }

    fn get_req_data(&self) -> Result<Vec<u8>, RpcError<String>> {
        // Try to get the request data
        debug!("Getting request data");

        // Return a 4MB vec
        Ok(vec![0u8; MAX_PACKET_SIZE])
    }

    fn set_resp_data(&mut self, _data: &[u8]) -> Result<(), RpcError<String>> {
        // Try to set the response data
        debug!("Setting response data");

        Ok(())
    }

    fn get_resp_data(&self) -> Result<Vec<u8>, RpcError<String>> {
        // Try to get the response data
        debug!("Getting response data");

        // Return a 4MB vec
        Ok(vec![0u8; MAX_PACKET_SIZE])
    }

    fn status(&self) -> PacketStatus {
        self.status
    }

    fn set_status(&mut self, status: PacketStatus) {
        self.status = status;
    }
}

pub struct TestHandler {
    done_tx: mpsc::Sender<Vec<u8>>,
}

impl TestHandler {
    pub fn new( done_tx: mpsc::Sender<Vec<u8>>) -> Self {
        Self { done_tx }
    }
}

#[async_trait]
impl Job for TestHandler {
    async fn run(&self) {
        self.done_tx.send(vec![0u8; 4]).await.unwrap();
    }
}

#[derive(Clone)]
pub struct TestRpcServerHandler {
    worker_pool: Arc<WorkerPool>,
}

impl TestRpcServerHandler {
    pub fn new(worker_pool: Arc<WorkerPool>) -> Self {
        Self { worker_pool }
    }
}

#[async_trait]
impl RpcServerConnectionHandler for TestRpcServerHandler {
    async fn dispatch(
        &self,
        req_header: ReqHeader,
        req_buffer: &[u8],
        done_tx: mpsc::Sender<Vec<u8>>,
    ) {
        // Dispatch the handler for the connection
        if let Ok(req_type) = ReqType::from_u8(req_header.op) {
            match req_type {
                ReqType::FileBlockRequest => {
                    // Try to read the request body
                    // Decode the request body

                    // File block request
                    // Submit the handler to the worker pool
                    // When the handler is done, send the response to the done channel
                    // Response need to contain the response header and body
                    // let handler = TestHandler::new(done_tx.clone());
                    // if let Ok(_) = self
                    //     .worker_pool
                    //     .submit_job(Box::new(handler))
                    //     .map_err(|err| {
                    //         debug!("Failed to submit job: {:?}", err);
                    //     })
                    // {
                    //     debug!("Submitted job to worker pool");
                    // }

                    // Create a response packet
                    let resp_body_packet = TestPacket::new(req_header.op).get_resp_data().unwrap();
                    let resp_header = RespHeader{
                        seq: req_header.seq,
                        op: req_header.op,
                        len: resp_body_packet.len() as u64,
                    };

                    let mut resp_packet = resp_header.encode();
                    resp_packet.extend(resp_body_packet);

                    done_tx.send(resp_packet).await.unwrap();
                }
                _ => {
                    debug!(
                        "FileBlockRpcServerHandler: Inner request type is not matched: {:?}",
                        req_header.op
                    );
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // Set the tracing log level to debug
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .finish(),
    )
    .expect("Failed to set tracing subscriber");

    let rpc_server_options = ServerTimeoutOptions {
        read_timeout: Duration::from_secs(10),
        write_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(10),
    };
    let rpc_client_options = ClientTimeoutOptions {
        read_timeout: Duration::from_secs(10),
        write_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(10),
        keep_alive_timeout: Duration::from_secs(10),
    };

    // Create server
    let addr = "127.0.0.1:2730";
    let pool: Arc<WorkerPool> = Arc::new(WorkerPool::new(1, 1));
    let handler = TestRpcServerHandler::new(pool.clone());
    // let handler = FileBlockRpcServerHandler::new(pool.clone());
    let mut server = RpcServer::new(rpc_server_options, 1, 1, handler);
    server.listen(addr).await.unwrap();

    // Check server is started
    tokio::time::sleep(Duration::from_secs(1)).await;
    // assert!(is_port_in_use(addr).await);

    let start = Instant::now();
    let duration = start.elapsed();
    info!("Time taken to send requests for {} files: cost {} s", MAX_PACKET_NUM, duration.as_secs_f64());

    let packet = TestPacket::new(1);
    // Create client
    let connect_stream = connect_timeout!(addr, rpc_client_options.read_timeout).await.unwrap();
    let rpc_client = RpcClient::<TestPacket>::new(connect_stream, rpc_client_options).await;

    for idx in 0..MAX_PACKET_NUM {
        debug!("Sending request: {}", idx);
        let res = rpc_client.send_request(&mut packet.clone()).await;
        // assert!(res.is_ok());
    }

    // check response
    // for idx in 0..MAX_PACKET_NUM {
    //     debug!("Receiving response: {}", idx);
    //     let res = rpc_client.recv_response().await;
    //     // assert!(res.is_ok());
    // }
    let mut final_packet = TestPacket::new(MAX_PACKET_NUM as u8);
    loop {
        match rpc_client.recv_response(&mut final_packet).await {
            Some(data) => {
                debug!("Receiving response: {:?}", data);
                if data.seq() >= MAX_PACKET_NUM as u64 {
                    break;
                }
            }
            None => {
                break;
            }
        }
    }

    let duration = start.elapsed();
    info!("Time taken to send and receive responses for {} files, cost {} s, buffer size is {}MB, speed is {} MB/s", MAX_PACKET_NUM, duration.as_secs_f64(), MAX_PACKET_SIZE / 1024 / 1024, ((MAX_PACKET_SIZE * MAX_PACKET_NUM / 1024 / 1024) as f64) / duration.as_secs_f64());

    // let resp = rpc_client.recv_response().await;

    // Wait for the server to start
    // tokio::time::sleep(Duration::from_secs(2)).await;

    drop(rpc_client);
    server.stop().await;
}
