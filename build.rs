use std::process::Command;
use tonic_build;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tell Cargo to rerun this if the proto file changes
    println!("cargo:rerun-if-changed=src/proto/metrics.proto");
    
    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional") // Enable proto3 optional fields
        .build_server(true)
        .build_client(true)
        .compile(
            &["src/proto/metrics.proto"],
            &["src/proto"], // The directory containing your .proto files
        )?;
    
    Ok(())
}
