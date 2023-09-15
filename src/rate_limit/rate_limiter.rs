use std::collections::HashMap;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// A basic rate limiter implementation.
#[derive(Clone, Debug)]
pub struct RateLimiter {
    ip_counts: Arc<Mutex<HashMap<SocketAddr, u32>>>, // IP address to (request count, last request time)
    max_requests: u32,                               // Maximum requests allowed per minute
    reset_duration: Duration,                        // Duration for resetting request counts
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            reset_duration: Duration::from_secs(60),
            max_requests: 100, // 100
            ip_counts: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl RateLimiter {
    pub fn new(max_requests: u32, reset_duration: Duration) -> Self {
        let limiter = Self {
            ip_counts: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            reset_duration,
        };
        log::info!("RateLimiter initialized!");

        let l = limiter.clone();
        std::thread::spawn(move || {
            println!("{:?} running RateLimiter", std::thread::current().id());

            loop {
                // Do things..
                l.ip_counts.clone().lock().unwrap().clear();

                std::thread::sleep(limiter.reset_duration);
                println!("RateLimiter reset...");
            }
        });

        limiter
    }

    pub fn check_rate_limit(&mut self, ip: SocketAddr) -> bool {
        if let Ok(mut lock) = self.ip_counts.lock() {
            if let Some(requests) = lock.get_mut(&ip) {
                *requests += 1;

                return requests < &mut self.max_requests;
            } else {
                lock.insert(ip, 1);
                return true;
            }
        }

        return false;
    }
}
