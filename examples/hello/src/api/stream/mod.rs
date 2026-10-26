use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static HITS: AtomicU64 = AtomicU64::new(0);

/// Push this route on `/ws` once per second. Any other route can do the same
/// with its own interval. The page does not mention the socket.
pub const STREAM_EVERY: Duration = Duration::from_secs(1);

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub message: String,
    pub count: u64,
}

pub fn get() -> Response {
    let count = HITS.fetch_add(1, Ordering::Relaxed) + 1;
    Response {
        message: "every second".into(),
        count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_call_advances_the_count() {
        assert_eq!(STREAM_EVERY, Duration::from_secs(1));
        let first = get().count;
        let second = get().count;
        assert!(second > first);
    }
}
