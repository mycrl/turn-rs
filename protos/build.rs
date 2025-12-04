fn main() {
    println!("cargo:rerun-if-changed=protobufs/");

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["./protobufs/server.proto"], &["."])
        .expect("failed to compile gRPC definitions");
}
