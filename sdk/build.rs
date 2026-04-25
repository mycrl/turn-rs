fn main() {
    println!("cargo:rerun-if-changed=protos/");

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["./protos/server.proto"], &["."])
        .expect("failed to compile gRPC definitions");
}
