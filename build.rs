fn main() {
    tonic_build::compile_protos("proto/file_transfer.proto").unwrap();
}
