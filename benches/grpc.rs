use tonic::transport::Server;
use tonic::{Request, Response, Status};
use benchmark::benchmark_service_server::{BenchmarkService, BenchmarkServiceServer};
use benchmark::{DataStream, Response as ProtoResponse};
use criterion::async_executor::FuturesExecutor;

pub mod benchmark {
    tonic::include_proto!("benchmark"); // 指定与 proto 文件中的 package 名称一致
}

pub struct MyBenchmarkService;

#[tonic::async_trait]
impl BenchmarkService for MyBenchmarkService {
    async fn send_data(
        &self,
        request: Request<DataStream>,
    ) -> Result<Response<ProtoResponse>, Status> {
        Ok(Response::new(ProtoResponse { success: true }))
    }
}

async fn grpc_benchmark() {
    let addr = "[::1]:50052".parse().unwrap();
    let service = MyBenchmarkService {};

    let _ = Server::builder()
        .add_service(BenchmarkServiceServer::new(service))
        .serve(addr)
        .await;
}

fn grpc_benchmark_criterion(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("grpc_benchmark", |b| {
        b.to_async(&FuturesExecutor).iter(|| grpc_benchmark());
    });
}
