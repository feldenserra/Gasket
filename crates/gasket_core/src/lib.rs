//! Axum engine for Gasket applications.
//!
//! [`serve_http`] serves the router it is given. [`serve`] also attaches a
//! `/ws` endpoint that speaks [`gasket_protocol`] messages. The shell uses
//! [`serve_http`] and mounts its own `/ws` for application JSON.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::routing::get;
use axum::Router;
use gasket_protocol::{ClientMessage, ServerMessage};
use tokio::net::TcpListener;

type StateHandler = Arc<dyn Fn(ClientMessage) -> ServerMessage + Send + Sync>;

/// Bind the engine to `127.0.0.1:port`.
///
/// Pass `0` to let the OS choose a port. Read [`TcpListener::local_addr`]
/// before [`serve`] so the UI can connect to the bound address.
pub async fn bind(port: u16) -> std::io::Result<TcpListener> {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    TcpListener::bind(addr).await
}

/// Serve `app` until the listener closes.
///
/// Does not attach a socket of its own. The shell mounts `/ws` when it builds
/// the application router.
pub async fn serve_http(listener: TcpListener, app: Router) -> std::io::Result<()> {
    axum::serve(listener, app).await
}

/// Serve `app` plus the `/ws` state stream until the listener closes.
///
/// `handler` is the application. It runs on the core runtime and must not block.
/// Core owns the socket; the application owns the reply. `/ws` is reserved by
/// the engine.
pub async fn serve<F>(listener: TcpListener, app: Router, handler: F) -> std::io::Result<()>
where
    F: Fn(ClientMessage) -> ServerMessage + Send + Sync + 'static,
{
    let handler: StateHandler = Arc::new(handler);
    let app = app.route(
        "/ws",
        get(move |ws: WebSocketUpgrade| {
            let handler = Arc::clone(&handler);
            async move { ws.on_upgrade(move |socket| handle_socket(socket, handler)) }
        }),
    );
    axum::serve(listener, app).await
}

async fn handle_socket(mut socket: WebSocket, handler: StateHandler) {
    while let Some(Ok(frame)) = socket.recv().await {
        let Message::Text(text) = frame else {
            continue;
        };
        let Ok(incoming) = serde_json::from_str::<ClientMessage>(&text) else {
            continue;
        };
        let outgoing = handler(incoming);
        let Ok(json) = serde_json::to_string(&outgoing) else {
            continue;
        };
        if socket.send(Message::Text(json)).await.is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn websocket_hello_roundtrip() {
        let listener = bind(0).await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            serve(listener, Router::new(), |_| ServerMessage::Hello {
                message: "hi".into(),
                status: "ok".into(),
            })
            .await
            .unwrap();
        });

        let frame = tokio::task::spawn_blocking(move || {
            let url = format!("ws://{addr}/ws");
            let (mut socket, _) = tungstenite::connect(url).unwrap();
            let payload = serde_json::to_string(&ClientMessage::RequestHello).unwrap();
            socket.send(tungstenite::Message::Text(payload)).unwrap();
            socket.read().unwrap()
        })
        .await
        .unwrap();

        let tungstenite::Message::Text(text) = frame else {
            panic!("expected a text frame, got {frame:?}");
        };
        let decoded: ServerMessage = serde_json::from_str(text.as_ref()).unwrap();
        assert_eq!(
            decoded,
            ServerMessage::Hello {
                message: "hi".into(),
                status: "ok".into(),
            }
        );
    }
}
