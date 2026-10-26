#[derive(serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub status: String,
}

pub fn get() -> Response {
    Response {
        status: "ok".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_is_ok() {
        assert_eq!(get().status, "ok");
    }
}
