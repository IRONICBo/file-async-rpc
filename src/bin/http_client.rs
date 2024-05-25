use hyper::{Client, Request, Body, Method};
use hyper::client::HttpConnector;
use tokio::runtime::Runtime;
use bytes::Bytes;
use std::time::Instant;

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::<HttpConnector>::new();
    let uri = "http://127.0.0.1:8990/".to_string();
    let num_blocks = 1000;
    let block_size = 4 * 1024 * 1024; // 4MB
    let data = Bytes::from(vec![0u8; block_size]);

    let start = Instant::now();
    for _ in 0..num_blocks {
        let req = Request::builder()
            .method(Method::POST)
            .uri(&uri)
            .body(Body::from(data.clone()))
            .expect("request builder");

        let resp = client.request(req).await?;
        hyper::body::to_bytes(resp.into_body()).await?;
    }
    let duration = start.elapsed();

    println!("Time taken to send and receive responses for {} files: {:?}, speed: {:?} MB/s", num_blocks, duration.as_secs_f64(), ((num_blocks as f64) * 4.0) / duration.as_secs_f64());
    Ok(())
}

fn main() {
    let rt = Runtime::new().unwrap();
    rt.block_on(run());
}
