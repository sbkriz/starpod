//! HTTP proxy request handling.
//!
//! Handles both plain HTTP requests (GET/POST/etc.) and CONNECT tunnels.
//! When the `mitm` feature is enabled, CONNECT tunnels are intercepted
//! with ephemeral TLS certificates for token scanning.

use std::sync::Arc;

use aes_gcm::Aes256Gcm;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;
use tracing::{debug, error, warn};

use crate::scan;
use crate::tunnel;

/// Shared proxy state.
pub(crate) struct ProxyState {
    pub cipher: Aes256Gcm,
    pub http_client: reqwest::Client,
    #[cfg(feature = "mitm")]
    pub ca: Option<Arc<crate::ca::CertAuthority>>,
    #[cfg(feature = "mitm")]
    pub cipher_arc: Arc<Aes256Gcm>,
}

/// Handle an incoming proxy request.
pub(crate) async fn handle_request(
    state: Arc<ProxyState>,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    if req.method() == Method::CONNECT {
        return handle_connect(state, req).await;
    }
    handle_http(state, req).await
}

/// Handle a plain HTTP proxy request (non-CONNECT).
async fn handle_http(
    state: Arc<ProxyState>,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let uri = req.uri().clone();
    let target_host = uri.host().unwrap_or("").to_string();

    debug!(
        method = %req.method(),
        uri = %uri,
        host = %target_host,
        "Proxying HTTP request"
    );

    let url_str = uri.to_string();

    // Collect body
    let body_bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!("Failed to read request body: {e}");
            return Ok(error_response(502, "Failed to read request body"));
        }
    };

    // Scan body for tokens
    let body_result = scan::scan_and_replace(&state.cipher, &body_bytes, &target_host);
    if body_result.replaced > 0 || body_result.stripped > 0 {
        debug!(
            replaced = body_result.replaced,
            stripped = body_result.stripped,
            "Tokens processed in request body"
        );
    }

    // Forward via reqwest
    let resp = match state
        .http_client
        .get(&url_str)
        .body(body_result.data)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("Upstream request failed: {e}");
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

/// Handle a CONNECT request (HTTPS tunnel).
async fn handle_connect(
    state: Arc<ProxyState>,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let authority = req
        .uri()
        .authority()
        .map(|a| a.to_string())
        .unwrap_or_default();

    // Parse host:port
    let (hostname, port) = if let Some(colon) = authority.rfind(':') {
        let host = &authority[..colon];
        let port = authority[colon + 1..].parse::<u16>().unwrap_or(443);
        (host.to_string(), port)
    } else {
        (authority.clone(), 443)
    };

    let addr = format!("{hostname}:{port}");
    debug!(addr = %addr, "CONNECT tunnel requested");

    // Spawn tunnel task after upgrade
    tokio::spawn(async move {
        let _state = state; // keep alive for mitm feature
        match hyper::upgrade::on(req).await {
            Ok(upgraded) => {
                let upgraded = TokioIo::new(upgraded);

                // Try MITM if CA is available
                #[cfg(feature = "mitm")]
                if let Some(ref ca) = _state.ca {
                    crate::mitm::handle_mitm(
                        upgraded,
                        hostname.clone(),
                        port,
                        Arc::clone(&_state.cipher_arc),
                        Arc::clone(ca),
                    )
                    .await;
                    return;
                }

                // Fallback: blind tunnel
                let target = match TcpStream::connect(&addr).await {
                    Ok(t) => t,
                    Err(e) => {
                        warn!(addr = %addr, error = %e, "Failed to connect to target");
                        return;
                    }
                };
                if let Err(e) = tunnel::tunnel_streams(upgraded, target).await {
                    if e.kind() != std::io::ErrorKind::NotConnected {
                        debug!(addr = %addr, error = %e, "Tunnel closed with error");
                    }
                }
            }
            Err(e) => {
                warn!("CONNECT upgrade failed: {e}");
            }
        }
    });

    Ok(Response::new(Full::new(Bytes::new())))
}

fn error_response(status: u16, msg: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .body(Full::new(Bytes::from(msg.to_string())))
        .unwrap()
}
