use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static HITS: AtomicU64 = AtomicU64::new(0);

pub const STREAM_EVERY: Duration = Duration::from_secs(5);

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub message: String,
    pub count: u64,
}

pub fn get() -> Response {
    let count = HITS.fetch_add(1, Ordering::Relaxed) + 1;
    Response {
        message: "every 5 seconds".into(),
        count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulse_is_slower_than_a_second() {
        assert_eq!(STREAM_EVERY, Duration::from_secs(5));
        assert!(get().count >= 1);
    }
}
