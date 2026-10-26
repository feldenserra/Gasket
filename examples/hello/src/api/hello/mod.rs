#[derive(serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub message: String,
    pub status: String,
}

pub fn get() -> Response {
    Response {
        message: "Hello from the Gasket reference app".into(),
        status: "ok".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_snapshot() {
        let response = get();
        assert_eq!(response.message, "Hello from the Gasket reference app");
        assert_eq!(response.status, "ok");
    }
}
