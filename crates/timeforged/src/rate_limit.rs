use axum::{
    extract::ConnectInfo,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct RateLimiter {
    state: Arc<Mutex<HashMap<IpAddr, Vec<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    pub fn check(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut map = self.state.lock().unwrap();
        let entries = map.entry(ip).or_default();
        entries.retain(|t| now.duration_since(*t) < self.window);
        if entries.len() >= self.max_requests {
            false
        } else {
            entries.push(now);
            true
        }
    }
}

/// Middleware: 10 requests per hour per IP (for registration)
pub async fn register_rate_limit(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    static LIMITER: std::sync::OnceLock<RateLimiter> = std::sync::OnceLock::new();
    let limiter = LIMITER.get_or_init(|| RateLimiter::new(10, Duration::from_secs(3600)));

    if !limiter.check(addr.ip()) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    Ok(next.run(req).await)
}

/// Middleware: 120 event writes per minute per IP
pub async fn event_rate_limit(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    static LIMITER: std::sync::OnceLock<RateLimiter> = std::sync::OnceLock::new();
    let limiter = LIMITER.get_or_init(|| RateLimiter::new(120, Duration::from_secs(60)));

    if !limiter.check(addr.ip()) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    Ok(next.run(req).await)
}
