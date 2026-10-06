use reqwest::Client;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;
use tokio::time::interval;

#[derive(Default)]
struct Stats {
    ok: AtomicU64,
    failed: AtomicU64,
}

async fn report(stats: Arc<Stats>) {
    let mut tick = interval(Duration::from_secs(1));
    loop {
        tick.tick().await;
        let ok = stats.ok.swap(0, Ordering::Relaxed);
        let failed = stats.failed.swap(0, Ordering::Relaxed);
        println!("ok {ok}/s, failed {failed}/s");
    }
}

pub async fn hammer(addr: String) {
    let client = Client::new();
    let url = format!("http://{addr}");
    let stats = Arc::new(Stats::default());
    tokio::spawn(report(stats.clone()));
    let mut tick = interval(Duration::from_millis(1));
    loop {
        tick.tick().await;
        for _ in 0..10 {
            let client = client.clone();
            let url = url.clone();
            let stats = stats.clone();
            tokio::spawn(async move {
                let result = client.get(url).send().await;
                let ok = matches!(result, Ok(res) if res.status().is_success());
                let counter = if ok { &stats.ok } else { &stats.failed };
                counter.fetch_add(1, Ordering::Relaxed);
            });
        }
    }
}
