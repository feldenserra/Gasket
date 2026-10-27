pub struct Response {
    pub title: String,
    pub body: String,
}

pub struct Note {
    pub text: String,
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

pub fn post(req: &gasket_ui::Request) -> Response {
    match req.json::<Note>() {
        Ok(note) if note.text.trim().is_empty() => Response {
            title: "About".into(),
            body: "Type a note first.".into(),
        },
        Ok(note) => Response {
            title: "About".into(),
            body: format!("Noted: {}", note.text.trim()),
        },
        Err(_) => Response {
            title: "About".into(),
            body: "That was not a note.".into(),
        },
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

    #[test]
    fn post_reads_a_typed_note() {
        let mut req = gasket_ui::Request::get("/api/about");
        req.body = br#"{"text":"hello"}"#.to_vec();
        let response = post(&req);
        assert_eq!(response.body, "Noted: hello");
    }
}
