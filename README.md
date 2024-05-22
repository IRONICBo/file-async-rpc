# File block transfer async rpc

### 1. Introduction

This project is a rpc for file block transfer async rpc, it includes two parts: client and server. The client send a file block to server, the server receive the file block and send back a response to client, and support keep-alive connection.

- client.rs is the client part, it send a file block to server.
- server.rs is the server part, it receive the file block from client and send back a response to client.
- error.rs is the error part, it define the error type of this project.
- packet.rs is the packet part, it define the packet trait and manage packet of the procject.
- message.rs is the message part, it define the message type of this project.
- common.rs is the common part, it define the common functions of this project, typically the timeout option of connection.
- workerpool.rs is the workerpool part, it define the workerpool of this project, typically the workerpool of server.

### Benchmark

Support benchmark for file block transfer async rpc, try to compare the performance of different async rpc frameworks.

senario:
client send a file block to server, server receive the file block and send back a response to client, the file block size is 4MB. calculate the time cost of this senario.

1. file-async-rpc-benchmark

TODO:

2. grpc-benchmark

Run server
```rs
cargo run --release --bin grpc_server
```

Run client
```rs
cargo run --release --bin grpc_client
```

```bash
cargo run --release --bin grpc_client
   Compiling grpc_file_transfer v0.1.0 (/Users/asklv/Projects/DatenlordIntern/grpc_file_transfer)
    Finished release [optimized] target(s) in 2.95s
     Running `/Users/asklv/Projects/DatenlordIntern/grpc_file_transfer/target/release/grpc_client`
Time taken to send and receive responses for 10,000 files: 39.557685208, speed: 1011.1815135206785MB/s
```

3. raw tcp benchmark

Run server
```rs
cargo run --release --bin tcp_server
```

Run client
```rs
cargo run --release --bin tcp_client
```

```bash
cargo run --release --bin tcp_client
   Compiling grpc_file_transfer v0.1.0 (/Users/asklv/Projects/DatenlordIntern/grpc_file_transfer)
    Finished release [optimized] target(s) in 0.53s
     Running `/Users/asklv/Projects/DatenlordIntern/grpc_file_transfer/target/release/tcp_client`
Time taken to send and receive responses for 10,000 files: 8.775378, speed: 4558.208204820351MB/s
```

4. arpc

Run test:

```bash
    Finished release [optimized] target(s) in 1.86s
warning: the following packages contain code that will be rejected by a future version of Rust: syn v0.14.9
note: to see what the problems were, use the option `--future-incompat-report`, or run `cargo report future-incompatibilities --id 1`
     Running `target/release/test_arpc_performance`
server receive one connection
client send 10000 write task, task write buffer size is 4096K, cost time:39.889029333 s, as 1002.7819846422834 MB/s
```