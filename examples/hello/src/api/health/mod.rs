pub struct Response {
    pub status: String,
}

pub fn get(_req: &gasket_ui::Request) -> Response {
    Response {
        status: "ok".into(),
    }
}

#[cfg(test)]
enum Kind {
    Named { n: u8 },
    Tuple(u8),
    Quiet,
}

#[cfg(test)]
struct Wrap<T> {
    value: T,
    kind: Kind,
}

#[cfg(test)]
struct Miles(u32);

#[cfg(test)]
struct Silent;

#[cfg(test)]
struct Borrowed<'a> {
    name: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_is_ok() {
        assert_eq!(get(&gasket_ui::Request::get("/api/health")).status, "ok");
    }

    #[test]
    fn shapes_match_serde_json() {
        assert_eq!(
            gasket_ui::serde_json::to_vec(&Kind::Quiet).unwrap(),
            b"\"Quiet\""
        );
        assert_eq!(
            gasket_ui::serde_json::to_vec(&Kind::Tuple(2)).unwrap(),
            b"{\"Tuple\":2}"
        );
        assert_eq!(
            gasket_ui::serde_json::to_vec(&Kind::Named { n: 1 }).unwrap(),
            b"{\"Named\":{\"n\":1}}"
        );
        let wrap = Wrap {
            value: "a".to_string(),
            kind: Kind::Quiet,
        };
        let bytes = gasket_ui::serde_json::to_vec(&wrap).unwrap();
        let back: Wrap<String> = gasket_ui::serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back.value, "a");
        assert_eq!(gasket_ui::serde_json::to_vec(&Miles(3)).unwrap(), b"3");
        assert_eq!(gasket_ui::serde_json::to_vec(&Silent).unwrap(), b"null");
        assert_eq!(
            gasket_ui::serde_json::to_vec(&Borrowed { name: "ada" }).unwrap(),
            b"{\"name\":\"ada\"}"
        );
    }
}
