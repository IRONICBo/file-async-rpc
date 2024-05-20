use file_async_rpc::common::TimeoutOptions;

#[tokio::test]
async fn test_rpc_send_recv() {
    let rpc_server_options = TimeoutOptions::default();
    let rpc_client_options = TimeoutOptions::default();
}
