use std::{sync::Arc, time::Duration};

use file_async_rpc::{client::RpcClient, common::TimeoutOptions, error::RpcError, message::ReqType, packet::{Packet, ReqHeader}, server::{FileBlockRpcServerHandler, RpcServer, RpcServerConnectionHandler}, workerpool::{Job, WorkerPool}};
use tokio::{net::TcpStream, sync::mpsc, time::Instant};
use tonic::async_trait;
use tracing::{debug, info};

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
    pub status: u8,
}

impl TestPacket {
    pub fn new(op: u8) -> Self {
        Self { seq:0, op, status:0 }
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

    fn serialize(&self) -> Result<Vec<u8>, RpcError<String>> {
        // Try to serialize the request packet to a byte array

        // Return a 4MB vec
        Ok(vec![0u8; 4 * 1024 * 1024])
    }

    fn deserialize(&mut self, _data: &[u8]) -> Result<(), RpcError<String>> {
        // Try to get data and deserialize to response packet
        debug!("Deserializing response packet");

        Ok(())
    }

    fn status(&self) -> u8 {
        self.status
    }

    fn set_status(&mut self, status: u8) {
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
        self.done_tx.send(vec![0u8; 8]).await.unwrap();
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
        req_buffer: Vec<u8>,
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
                    let handler = TestHandler::new(done_tx.clone());
                    if let Ok(_) = self
                        .worker_pool
                        .submit_job(Box::new(handler))
                        .map_err(|err| {
                            debug!("Failed to submit job: {:?}", err);
                        })
                    {
                        debug!("Submitted job to worker pool");
                    }
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
            .with_max_level(tracing::Level::INFO)
            .finish(),
    )
    .expect("Failed to set tracing subscriber");

    let rpc_server_options = TimeoutOptions {
        read_timeout: Duration::from_secs(10),
        write_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(10),
    };
    let rpc_client_options = TimeoutOptions {
        read_timeout: Duration::from_secs(10),
        write_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(10),
    };

    // Create server
    let addr = "127.0.0.1:2730";
    let pool: Arc<WorkerPool> = Arc::new(WorkerPool::new(1000, 1000));
    let handler = TestRpcServerHandler::new(pool.clone());
    // let handler = FileBlockRpcServerHandler::new(pool.clone());
    let mut server = RpcServer::new(rpc_server_options, 4, 100, handler);
    server.listen(addr).await.unwrap();

    // Check server is started
    tokio::time::sleep(Duration::from_secs(1)).await;
    // assert!(is_port_in_use(addr).await);

    let start = Instant::now();
    let num_files = 1000;
    // Create client
    let rpc_client = RpcClient::<TestPacket>::new(addr, rpc_client_options).await;

    for idx in 0..num_files {
        debug!("Sending request: {}", idx);
        let res = rpc_client.send_request(TestPacket::new(1)).await;
        // assert!(res.is_ok());
    }


    // check response
    for idx in 0..num_files {
        debug!("Receiving response: {}", idx);
        let res = rpc_client.recv_response().await;
        // assert!(res.is_ok());
    }

    let duration = start.elapsed().as_micros();
    info!("Time taken to send and receive responses for {} files: cost {} ms", num_files, duration);

    // let resp = rpc_client.recv_response().await;

    // Wait for the server to start
    // tokio::time::sleep(Duration::from_secs(2)).await;

    server.stop().await;
}
