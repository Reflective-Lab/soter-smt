fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let proto_dir = manifest_dir.join("../../proto");
    let proto = proto_dir.join("soter.v1.proto");

    println!("cargo:rerun-if-changed={}", proto.display());

    // Propagate rpath from cvc5-sys so the runtime binary finds libcvc5
    // alongside libtonic etc. cvc5-sys's links="cvc5" emits DEP_CVC5_LIB_DIR.
    if let Ok(lib_dir) = std::env::var("DEP_CVC5_LIB_DIR") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{lib_dir}");
    }

    // Emit the encoded FileDescriptorSet for tonic-reflection.
    let descriptor_path =
        std::path::PathBuf::from(std::env::var("OUT_DIR")?).join("soter_descriptor.bin");

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(false)
        .file_descriptor_set_path(&descriptor_path)
        .compile_protos(&[proto], &[proto_dir])?;

    Ok(())
}
