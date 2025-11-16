fn main() {
    println!("cargo:rerun-if-changed=server.proto");

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["server.proto"], &["."])
        .expect("failed to compile gRPC definitions");
}
