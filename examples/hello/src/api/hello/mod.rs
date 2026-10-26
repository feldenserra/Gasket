#[derive(serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub message: String,
    pub status: String,
}

pub fn get(_req: &gasket_ui::Request) -> Response {
    Response {
        message: "Hello from the Gasket reference app".into(),
        status: "ok".into(),
    }
}

pub fn post(req: &gasket_ui::Request) -> Response {
    match req.json::<gasket_ui::serde_json::Value>() {
        Ok(body) => Response {
            message: body
                .get("message")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string(),
            status: "posted".into(),
        },
        Err(_) => Response {
            message: String::new(),
            status: "bad json".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_snapshot() {
        let response = get(&gasket_ui::Request::get("/api/hello"));
        assert_eq!(response.message, "Hello from the Gasket reference app");
        assert_eq!(response.status, "ok");
    }

    #[test]
    fn post_echoes_the_json_message() {
        let mut req = gasket_ui::Request::get("/api/hello");
        req.method = "POST".into();
        req.body = br#"{"message":"hi"}"#.to_vec();
        let response = post(&req);
        assert_eq!(response.status, "posted");
        assert_eq!(response.message, "hi");
    }
}
