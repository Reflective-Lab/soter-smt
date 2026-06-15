mod convert;
mod service;

pub mod proto {
    pub mod soter {
        #[allow(
            clippy::doc_markdown,
            clippy::default_trait_access,
            clippy::too_many_lines,
            clippy::large_enum_variant
        )]
        pub mod v1 {
            tonic::include_proto!("soter.v1");
        }
    }
}

fn main() {
    eprintln!("soter-server stub — wire main() in M2.A5");
    std::process::exit(1);
}
