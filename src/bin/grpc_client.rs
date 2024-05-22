use tonic::transport::Channel;
use file_transfer::file_transfer_service_client::FileTransferServiceClient;
use file_transfer::File;
use tokio::runtime::Runtime;
use std::time::Instant;

pub mod file_transfer {
    tonic::include_proto!("filetransfer"); // 指定与 .proto 文件中的 package 相对应
}

async fn send_file(client: &mut FileTransferServiceClient<Channel>, file_content: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let request = tonic::Request::new(File {
        content: file_content,
    });

    let _response = client.upload_file(request).await?;
    Ok(())
}

fn main() {
    let runtime = Runtime::new().unwrap();
    runtime.block_on(async {
        let mut client = FileTransferServiceClient::connect("http://127.0.0.1:50051").await.unwrap();
        let num_files = 10_000;
        let file_size = 4 * 1024 * 1024; // 4MB

        let start = Instant::now();

        for _ in 0..num_files {
            let file_content = vec![0u8; file_size];
            send_file(&mut client, file_content).await.unwrap();
        }

        let duration = start.elapsed();
        println!("Time taken to send and receive responses for 10,000 files: {:?}, speed: {:?}MB/s", duration.as_secs_f64(), ((num_files * 4) as f64) / duration.as_secs_f64());
    });
}


// use tonic::transport::Channel;
// use tonic::Request;

// use file_transfer::file_transfer_service_client::FileTransferServiceClient;
// use file_transfer::File;

// pub mod file_transfer {
//     tonic::include_proto!("filetransfer"); // 指定与 .proto 文件中的 package 相对应
// }

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let mut client = FileTransferServiceClient::connect("http://[::1]:50051").await?;

//     let file_content = vec![0u8; 4 * 1024 * 1024]; // 生成一个4MB的文件内容

//     let request = Request::new(File {
//         content: file_content,
//     });

//     let response = client.upload_file(request).await?;

//     println!("RESPONSE={:?}", response.into_inner().message);

//     Ok(())
// }
