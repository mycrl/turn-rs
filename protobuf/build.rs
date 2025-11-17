fn main() {
    println!("cargo:rerun-if-changed=protos/server.proto");
    println!("cargo:rerun-if-changed=protos/cluster.proto");

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["protos/server.proto", "protos/cluster.proto"], &["./protos"])
        .expect("failed to compile gRPC definitions");
}
