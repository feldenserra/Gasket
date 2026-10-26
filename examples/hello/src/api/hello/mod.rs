#[derive(serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub message: String,
    pub status: String,
}

pub fn get(_req: &gasket_ui::Request) -> Response {
    Response {
        message: "Type a name and submit.".into(),
        status: "ok".into(),
    }
}

pub fn post(req: &gasket_ui::Request) -> Response {
    match req.json::<gasket_ui::serde_json::Value>() {
        Ok(body) => {
            let name = body
                .get("name")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim();
            if name.is_empty() {
                Response {
                    message: "Type a name first.".into(),
                    status: "ok".into(),
                }
            } else {
                Response {
                    message: format!("I wish you the best, {name}"),
                    status: "ok".into(),
                }
            }
        }
        Err(_) => Response {
            message: "That was not JSON.".into(),
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
        assert_eq!(response.message, "Type a name and submit.");
        assert_eq!(response.status, "ok");
    }

    #[test]
    fn post_wishes_the_name_well() {
        let mut req = gasket_ui::Request::get("/api/hello");
        req.method = "POST".into();
        req.body = br#"{"name":"Ada"}"#.to_vec();
        let response = post(&req);
        assert_eq!(response.status, "ok");
        assert_eq!(response.message, "I wish you the best, Ada");
    }
}
