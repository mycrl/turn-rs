fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut prost_build = prost_build::Config::new();
    prost_build.protoc_arg("--experimental_allow_proto3_optional");
    prost_build.compile_protos(&["../../protos/balance.proto"], &["../../protos"])?;

    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile(&["../../protos/turn.proto"], &["../../protos"])?;
    Ok(())
}
