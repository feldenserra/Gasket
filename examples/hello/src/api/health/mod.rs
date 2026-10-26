#[derive(serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub status: String,
}

pub fn get(_req: &gasket_ui::Request) -> Response {
    Response {
        status: "ok".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_is_ok() {
        assert_eq!(get(&gasket_ui::Request::get("/api/health")).status, "ok");
    }
}
