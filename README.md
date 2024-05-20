# File block transfer async rpc

### 1. Introduction

This project is a rpc for file block transfer async rpc, it includes two parts: client and server. The client send a file block to server, the server receive the file block and send back a response to client, and support keep-alive connection.

- client.rs is the client part, it send a file block to server.
- server.rs is the server part, it receive the file block from client and send back a response to client.
- error.rs is the error part, it define the error type of this project.
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
TODO: