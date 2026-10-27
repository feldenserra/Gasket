pub struct Link {
    pub page: String,
    pub title: String,
}

pub struct Response {
    pub title: String,
    pub links: Vec<Link>,
    pub note: String,
}

pub fn get(_req: &gasket_ui::Request) -> Response {
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
        let response = get(&gasket_ui::Request::get("/api/index"));
        assert_eq!(response.title, "Gasket");
        assert_eq!(response.links.len(), 4);
        assert_eq!(response.links[0].page, "hello");
        assert_eq!(response.links[1].page, "about");
        assert_eq!(response.links[2].page, "stream");
        assert_eq!(response.links[3].page, "pulse");
        assert!(response.note.contains("/api/health"));
        let bytes = gasket_ui::serde_json::to_vec(&response).unwrap();
        let back: Response = gasket_ui::serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back.links[0].page, "hello");
    }
}
