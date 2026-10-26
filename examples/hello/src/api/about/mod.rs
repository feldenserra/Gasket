#[derive(serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub title: String,
    pub body: String,
}

pub fn get(req: &gasket_ui::Request) -> Response {
    let name = req.query("name").unwrap_or("there");
    Response {
        title: "About".into(),
        body: format!(
            "Hello, {name}. This route has no STREAM_EVERY, so opening the page is one HTTP GET."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_is_a_single_snapshot() {
        let response = get(&gasket_ui::Request::get("/api/about"));
        assert_eq!(response.title, "About");
        assert!(response.body.contains("Hello, there"));
        let mut named = gasket_ui::Request::get("/api/about");
        named.query.push(("name".into(), "Ada".into()));
        assert!(get(&named).body.contains("Hello, Ada"));
    }
}
