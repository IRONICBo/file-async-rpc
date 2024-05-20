use std::{sync::Arc, time::Duration};

use file_async_rpc::{client::RpcClient, common::TimeoutOptions, server::{FileBlockRpcServerHandler, RpcServer}, workerpool::WorkerPool};
use tokio::net::TcpStream;

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

#[tokio::test]
async fn test_rpc_send_recv() {
    // Set the tracing log level to debug
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
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
    let pool = Arc::new(WorkerPool::new(4, 100));
    let handler = FileBlockRpcServerHandler::new(pool.clone());
    let mut server = RpcServer::new(rpc_server_options, 4, 100, handler);
    server.listen(addr).await.unwrap();

    // Check server is started
    tokio::time::sleep(Duration::from_secs(1)).await;
    // assert!(is_port_in_use(addr).await);

    // Create client
    let rpc_client = RpcClient::new(addr, rpc_client_options).await;
    // let resp = rpc_client.recv_response().await;

    // Wait for the server to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    server.stop().await;
}
