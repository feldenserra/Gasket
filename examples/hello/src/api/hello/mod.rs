#[derive(serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub message: String,
    pub status: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Wish {
    pub name: String,
}

pub fn get(_req: &gasket_ui::Request) -> Response {
    Response {
        message: "Type a name and submit.".into(),
        status: "ok".into(),
    }
}

pub fn post(req: &gasket_ui::Request) -> Response {
    let wish = match req.json::<Wish>() {
        Ok(wish) => wish,
        Err(_) => {
            return Response {
                message: "That was not a name.".into(),
                status: "bad json".into(),
            };
        }
    };
    let name = wish.name.trim();
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
        assert_eq!(response.message, "I wish you the best, Ada");
    }
}
