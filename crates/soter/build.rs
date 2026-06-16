//! Build script for `converge-soter-smt`.
//!
//! Compiles the `soter.v1` gRPC client stubs ONLY when the `remote` feature
//! is on. Default-features builds skip this entirely, keeping the dep tree
//! small for consumers that only need the `SmtBackend` trait + `Cvc5FfiBackend`.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("CARGO_FEATURE_REMOTE").is_some() {
        let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
        let proto_dir = manifest_dir.join("../../proto");
        let proto = proto_dir.join("soter.v1.proto");
        println!("cargo:rerun-if-changed={}", proto.display());

        tonic_prost_build::configure()
            .build_server(false)
            .build_client(true)
            .compile_protos(&[proto], &[proto_dir])?;
    }
    Ok(())
}
