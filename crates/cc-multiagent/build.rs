fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Only compile protos if grpc feature is enabled
    #[cfg(feature = "grpc")]
    {
        tonic_build::configure()
            .build_server(true)
            .build_client(true)
            .out_dir("src/generated")
            .compile(
                &["proto/multiagent.proto"],
                &["proto"],
            )?;
    }
    
    Ok(())
}