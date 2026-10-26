//! Axum engine for Gasket applications.
//!
//! [`serve`] listens on the address the application configured and serves the
//! router the shell built. Application WebSocket subscriptions are part of
//! that router.

use std::net::SocketAddr;

use axum::Router;
use tokio::net::TcpListener;

/// Bind the engine to `addr`.
///
/// Pass port `0` to let the OS choose a port. Read [`TcpListener::local_addr`]
/// before [`serve`].
pub async fn bind(addr: SocketAddr) -> std::io::Result<TcpListener> {
    TcpListener::bind(addr).await
}

/// Serve `app` until the listener closes.
pub async fn serve(listener: TcpListener, app: Router) -> std::io::Result<()> {
    axum::serve(listener, app).await
}
