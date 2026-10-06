mod limiter;
mod router;

use limiter::Limiter;
use router::{hammer::hammer, router};
use std::env;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

fn env_or(key: &str, default: f64) -> f64 {
    match env::var(key) {
        Ok(v) => v
            .parse()
            .unwrap_or_else(|_| panic!("{key} must be a number")),
        Err(_) => default,
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    dotenvy::dotenv().ok();
    let port = env::var("PORT").unwrap_or("8888".into());
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    println!("listening on {}", listener.local_addr()?);
    let limiter = Arc::new(Limiter::new(
        env_or("RATE_BURST", 100.0),
        env_or("RATE_PER_SEC", 1000.0),
    ));
    tokio::spawn(limiter::sweep(limiter.clone()));
    let app = router(limiter);
    tokio::spawn(hammer(listener.local_addr()?.to_string()));
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
