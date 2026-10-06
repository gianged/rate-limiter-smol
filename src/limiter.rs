use axum::{
    extract::{ConnectInfo, Request, State},
    http::{StatusCode, header::RETRY_AFTER},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};
use tokio::time::interval;

const IDLE_TTL: Duration = Duration::from_secs(60);

struct Bucket {
    tokens: f64,
    last: Instant,
}

pub struct Limiter {
    buckets: Mutex<HashMap<IpAddr, Bucket>>,
    capacity: f64,
    rate: f64,
}

impl Limiter {
    pub fn new(capacity: f64, rate: f64) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            capacity,
            rate,
        }
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<IpAddr, Bucket>> {
        self.buckets.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn allow(&self, ip: IpAddr) -> Result<(), f64> {
        let now = Instant::now();
        let mut buckets = self.lock();
        let bucket = buckets.entry(ip).or_insert(Bucket {
            tokens: self.capacity,
            last: now,
        });
        let elapsed = now.duration_since(bucket.last).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.rate).min(self.capacity);
        bucket.last = now;
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(())
        } else {
            Err((1.0 - bucket.tokens) / self.rate)
        }
    }

    pub fn evict(&self) {
        let now = Instant::now();
        self.lock()
            .retain(|_, bucket| now.duration_since(bucket.last) < IDLE_TTL);
    }
}

pub async fn limit(
    State(limiter): State<Arc<Limiter>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    match limiter.allow(addr.ip()) {
        Ok(()) => next.run(req).await,
        Err(wait) => {
            let secs = wait.ceil() as u64;
            (
                StatusCode::TOO_MANY_REQUESTS,
                [(RETRY_AFTER, secs.to_string())],
            )
                .into_response()
        }
    }
}

pub async fn sweep(limiter: Arc<Limiter>) {
    let mut tick = interval(Duration::from_secs(10));
    loop {
        tick.tick().await;
        limiter.evict();
    }
}
