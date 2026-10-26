#[derive(serde::Serialize, serde::Deserialize)]
pub struct Link {
    pub page: String,
    pub title: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub title: String,
    pub links: Vec<Link>,
    pub note: String,
}

pub fn get() -> Response {
    Response {
        title: "Gasket".into(),
        links: vec![
            Link {
                page: "hello".into(),
                title: "Hello".into(),
            },
            Link {
                page: "about".into(),
                title: "About".into(),
            },
            Link {
                page: "stream".into(),
                title: "Stream, every second".into(),
            },
            Link {
                page: "pulse".into(),
                title: "Pulse, every 5 seconds".into(),
            },
        ],
        note: "GET /api/health has no page. Curl it.".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_lists_the_other_pages() {
        let response = get();
        assert_eq!(response.title, "Gasket");
        assert_eq!(response.links.len(), 4);
        assert_eq!(response.links[0].page, "hello");
        assert_eq!(response.links[1].page, "about");
        assert_eq!(response.links[2].page, "stream");
        assert_eq!(response.links[3].page, "pulse");
        assert!(response.note.contains("/api/health"));
    }
}
