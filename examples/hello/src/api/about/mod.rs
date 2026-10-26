#[derive(serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub title: String,
    pub body: String,
}

pub fn get() -> Response {
    Response {
        title: "About".into(),
        body: "This route has no STREAM_EVERY, so opening the page is one HTTP GET.".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_is_a_single_snapshot() {
        let response = get();
        assert_eq!(response.title, "About");
        assert!(response.body.contains("STREAM_EVERY"));
    }
}
