fn main() {
    #[cfg(feature = "rpc")]
    {
        tonic_prost_build::configure()
            .build_server(true)
            .build_client(true)
            .compile_protos(&["protos/server.proto"], &["protos"])
            .unwrap();
    }
}
