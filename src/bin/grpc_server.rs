use tonic::{transport::Server, Request, Response, Status};

use file_transfer::file_transfer_service_server::{FileTransferService, FileTransferServiceServer};
use file_transfer::{File, UploadStatus};

pub mod file_transfer {
    tonic::include_proto!("filetransfer"); // 指定与 .proto 文件中的 package 相对应
}

#[derive(Default)]
pub struct MyFileTransfer {}

#[tonic::async_trait]
impl FileTransferService for MyFileTransfer {
    async fn upload_file(&self, request: Request<File>) -> Result<Response<UploadStatus>, Status> {
        // println!("Received a file with {} bytes", request.into_inner().content.len());

        Ok(Response::new(UploadStatus {
            message: "File uploaded successfully.".into(),
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:50051".parse()?;
    let service = MyFileTransfer::default();

    Server::builder()
        .add_service(FileTransferServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
