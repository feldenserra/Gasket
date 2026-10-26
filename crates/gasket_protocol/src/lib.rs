//! Message types for `gasket_core::serve`'s protocol socket.
//!
//! Application pages do not use this crate. A page's model is the `Response`
//! struct in its route module. The shell's `/ws` subscription carries that
//! route's JSON, not [`ClientMessage`] or [`ServerMessage`].

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
