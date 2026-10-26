//! Shared wire types for the Gasket state stream.
//!
//! `gasket_core` and `gasket_ui` both speak this crate. Neither defines the
//! contract itself, and the UI crate does not depend on the engine.
//!
//! WebSocket text frames on `/ws` are JSON [`ClientMessage`] and
//! [`ServerMessage`] values. Application HTTP routes may use the same types,
//! but those routes are not the state stream.

use serde::{Deserialize, Serialize};

/// A message the native client sends to core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Ask the application for its hello snapshot.
    RequestHello,
}

/// A message core sends to the native client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Application hello snapshot.
    Hello { message: String, status: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_message_roundtrip() {
        let message = ClientMessage::RequestHello;
        let json = serde_json::to_string(&message).unwrap();
        assert_eq!(json, r#"{"type":"request_hello"}"#);
        let decoded: ClientMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, message);
    }

    #[test]
    fn server_message_roundtrip() {
        let message = ServerMessage::Hello {
            message: "hi".into(),
            status: "ok".into(),
        };
        let json = serde_json::to_string(&message).unwrap();
        assert_eq!(json, r#"{"type":"hello","message":"hi","status":"ok"}"#);
        let decoded: ServerMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, message);
    }
}
