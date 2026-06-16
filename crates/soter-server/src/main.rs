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

use std::net::SocketAddr;
use std::sync::Arc;

use tonic::transport::{Identity, Server, ServerTlsConfig};

use converge_soter_server::interceptor::request_interceptor;
use converge_soter_server::tenants::TenantRegistry;
use proto::soter::v1::soter_solver_server::SoterSolverServer;
use service::SoterSolverService;

/// Encoded `FileDescriptorSet` for `soter.v1`. Emitted by `build.rs` via
/// `tonic_prost_build::configure().file_descriptor_set_path(...)`. Consumed
/// by `tonic-reflection` to advertise the service schema over the standard
/// `grpc.reflection.v1.ServerReflection` API.
const SOTER_FILE_DESCRIPTOR_SET: &[u8] =
    tonic::include_file_descriptor_set!("soter_descriptor");

// The production image is always built with --features full (CVC5 linked).
// A default-feature-off build is fine for `cargo check` smoke tests on
// hosts without CVC5, but it would emit a binary that has no real backend,
// so the bin gate is here. The lib (`converge_soter_server`) stays
// feature-agnostic so unit tests on interceptor/tenants still build.
#[cfg(not(feature = "full"))]
compile_error!(
    "soter-server requires --features full (CVC5 linked); see Cargo.toml"
);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_current_span(true)
        .with_span_list(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "soter_server=info".parse().unwrap()),
        )
        .init();

    let addr: SocketAddr = std::env::var("SOTER_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50051".into())
        .parse()?;

    // Build the production backend. The `compile_error!` above guarantees
    // `full` is enabled here, so the cfg-gate is just a belt-and-braces
    // alias for the same condition.
    #[cfg(feature = "full")]
    let backend: Arc<dyn soter::SmtBackend> = Arc::new(soter::Cvc5FfiBackend);

    let svc = SoterSolverService::new(backend, Arc::new(TenantRegistry::default()));

    // Health checking — standard grpc.health.v1.Health, exposed without
    // tenant gating so Cloud Run probes and grpcurl can hit it freely.
    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<SoterSolverServer<SoterSolverService>>()
        .await;

    // gRPC reflection — exposes grpc.reflection.v1.ServerReflection so
    // grpcurl / Postman / Buf CLI can introspect the service without a
    // local .proto file. Registers both the soter.v1 descriptor (emitted
    // by build.rs) and tonic-health's bundled descriptor so `grpcurl list`
    // returns both services.
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(tonic_health::pb::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(SOTER_FILE_DESCRIPTOR_SET)
        .build_v1()?;

    let cert_path = std::env::var("SOTER_TLS_CERT").unwrap_or_else(|_| "/tls/server.crt".into());
    let key_path = std::env::var("SOTER_TLS_KEY").unwrap_or_else(|_| "/tls/server.key".into());

    let use_tls =
        std::path::Path::new(&cert_path).exists() && std::path::Path::new(&key_path).exists();

    tracing::info!(addr = %addr, tls = use_tls, "soter-server starting");

    if use_tls {
        let cert = std::fs::read(&cert_path)?;
        let key = std::fs::read(&key_path)?;
        let identity = Identity::from_pem(cert, key);
        let mut tls = ServerTlsConfig::new().identity(identity);

        if let Ok(ca_path) = std::env::var("SOTER_TLS_CA") {
            let ca = std::fs::read(ca_path)?;
            tls = tls.client_ca_root(tonic::transport::Certificate::from_pem(ca));
        }

        Server::builder()
            .tls_config(tls)?
            .add_service(health_service)
            .add_service(reflection_service)
            .add_service(SoterSolverServer::with_interceptor(
                svc,
                request_interceptor,
            ))
            .serve(addr)
            .await?;
    } else {
        tracing::warn!("TLS cert/key not found — starting without TLS (dev/test only)");
        Server::builder()
            .add_service(health_service)
            .add_service(reflection_service)
            .add_service(SoterSolverServer::with_interceptor(
                svc,
                request_interceptor,
            ))
            .serve(addr)
            .await?;
    }

    Ok(())
}
