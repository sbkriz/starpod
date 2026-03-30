//! TLS Man-in-the-Middle handler for HTTPS CONNECT tunnels.
//!
//! When enabled, the proxy intercepts HTTPS CONNECT requests by:
//! 1. Generating an ephemeral TLS certificate for the target hostname
//! 2. Accepting the client's TLS connection using that certificate
//! 3. Opening a real TLS connection to the target server
//! 4. Relaying HTTP requests between them, scanning for opaque tokens

use std::sync::Arc;

use aes_gcm::Aes256Gcm;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::TlsAcceptor;
use tracing::{debug, warn};

use crate::ca::CertAuthority;
use crate::scan;

/// Handle a CONNECT tunnel with MITM TLS interception.
///
/// `upgraded` is the raw TCP stream from the client after the HTTP CONNECT
/// handshake. We terminate TLS with an ephemeral cert, relay requests to
/// the real server, and scan all traffic for opaque tokens.
pub async fn handle_mitm(
    upgraded: impl AsyncRead + AsyncWrite + Unpin + Send + 'static,
    hostname: String,
    port: u16,
    cipher: Arc<Aes256Gcm>,
    ca: Arc<CertAuthority>,
) {
    if let Err(e) = run_mitm(upgraded, &hostname, port, cipher, ca).await {
        debug!(host = %hostname, error = %e, "MITM session ended");
    }
}

async fn run_mitm(
    upgraded: impl AsyncRead + AsyncWrite + Unpin + Send + 'static,
    hostname: &str,
    port: u16,
    cipher: Arc<Aes256Gcm>,
    ca: Arc<CertAuthority>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1. Issue ephemeral cert for the target hostname
    let (cert_chain, key) = ca.issue_cert(hostname)?;

    let server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)?;

    // 2. Accept TLS from the client using our ephemeral cert
    let acceptor = TlsAcceptor::from(Arc::new(server_config));
    let client_tls = acceptor.accept(upgraded).await?;
    debug!(host = %hostname, "MITM TLS accepted from client");

    // 3. Serve HTTP/1.1 on the MITM'd connection, forwarding to the real server
    let target_host = hostname.to_string();
    let target_port = port;

    let io = TokioIo::new(client_tls);
    http1::Builder::new()
        .preserve_header_case(true)
        .title_case_headers(true)
        .serve_connection(
            io,
            service_fn(move |req| {
                let cipher = Arc::clone(&cipher);
                let host = target_host.clone();
                let port = target_port;
                async move { forward_request(req, &host, port, &cipher).await }
            }),
        )
        .await?;

    Ok(())
}

/// Forward a single HTTP request to the real target over TLS, scanning for tokens.
async fn forward_request(
    req: Request<Incoming>,
    target_host: &str,
    target_port: u16,
    cipher: &Aes256Gcm,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");

    debug!(
        method = %method,
        host = %target_host,
        path = %path,
        "MITM forwarding request"
    );

    // Read and scan the body
    let body_bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            warn!("Failed to read MITM request body: {e}");
            return Ok(error_response(502, "Failed to read request body"));
        }
    };

    let body_result = scan::scan_and_replace(cipher, &body_bytes, target_host);
    if body_result.replaced > 0 || body_result.stripped > 0 {
        debug!(
            replaced = body_result.replaced,
            stripped = body_result.stripped,
            host = %target_host,
            "Tokens processed in HTTPS request"
        );
    }

    // Forward to real server via reqwest (which handles TLS to the target)
    let url = format!("https://{target_host}:{target_port}{path}");
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap_or_default();

    let resp = match client
        .request(method.clone(), &url)
        .body(body_result.data)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!(host = %target_host, error = %e, "MITM upstream request failed");
            return Ok(error_response(502, &format!("Upstream error: {e}")));
        }
    };

    let status = resp.status();
    let resp_body = resp.bytes().await.unwrap_or_default();

    Ok(Response::builder()
        .status(status.as_u16())
        .body(Full::new(resp_body))
        .unwrap_or_else(|_| error_response(500, "Internal proxy error")))
}

fn error_response(status: u16, msg: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .body(Full::new(Bytes::from(msg.to_string())))
        .unwrap()
}
