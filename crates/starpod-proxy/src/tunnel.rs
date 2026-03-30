//! HTTPS CONNECT tunnel handling.
//!
//! Phase 2: blind tunnel (bidirectional byte copy, no scanning).
//! Phase 3 (mitm feature): MITM with ephemeral certs and token scanning.

use tokio::net::TcpStream;
use tracing::debug;

/// Bidirectional tunnel between an upgraded connection and a target TCP stream.
///
/// Copies bytes in both directions without inspection. HTTPS traffic remains
/// encrypted and opaque — no token scanning occurs in this mode.
pub async fn tunnel_streams(
    mut upgraded: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    mut target: TcpStream,
) -> std::io::Result<()> {
    let (bytes_to_target, bytes_from_target) =
        tokio::io::copy_bidirectional(&mut upgraded, &mut target).await?;
    debug!(
        to_target = bytes_to_target,
        from_target = bytes_from_target,
        "CONNECT tunnel closed"
    );
    Ok(())
}
