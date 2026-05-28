use std::sync::LazyLock;
use std::time::Duration;

pub static SHARED_HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(60))
        .pool_idle_timeout(Duration::from_secs(90))
        .build()
        .unwrap_or_default()
});
