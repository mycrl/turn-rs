fn main() {
    prost_build::compile_protos(&["../../protos/balance.proto"], &["../../protos"]).unwrap();
}
