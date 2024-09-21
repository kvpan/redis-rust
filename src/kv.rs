use std::collections::HashMap;
use std::sync::{Mutex, RwLock};
use std::time::Duration;

use anyhow::Context;
use tokio::sync::{mpsc, OnceCell};
use tokio::time::Instant;

static KV: OnceCell<RwLock<HashMap<String, String>>> = OnceCell::const_new();
static CRON: OnceCell<Mutex<Vec<(String, Instant)>>> = OnceCell::const_new();

pub fn init() {
    KV.set(RwLock::new(HashMap::new()))
        .expect("KV should be set only once");
    CRON.set(Mutex::new(Vec::new()))
        .expect("CRON should be set only once");

    tokio::spawn(async {
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;

            let mut cron = CRON
                .get()
                .expect("CRON should be initialized")
                .lock()
                .expect("Failed to acquire lock");

            let now = Instant::now();
            cron.retain(|(key, expiry)| {
                if now >= *expiry {
                    let mut kv = KV
                        .get()
                        .expect("KV should be initialized")
                        .write()
                        .expect("Failed to acquire write lock");
                    kv.remove(key);
                    false
                } else {
                    true
                }
            });
        }
    });
}

pub async fn get(key: &str) -> Option<String> {
    let kv = KV
        .get()
        .expect("KV should be initialized")
        .read()
        .expect("Failed to acquire read lock");

    kv.get(key).cloned()
}

pub async fn set(key: &str, value: String, expiry: Option<u64>) {
    let mut kv = KV
        .get()
        .expect("KV should be initialized")
        .write()
        .expect("Failed to acquire write lock");

    kv.insert(key.to_string(), value);

    if let Some(expiry) = expiry {
        let expiry = Instant::now() + Duration::from_millis(expiry);
        let mut cron = CRON
            .get()
            .expect("CRON should be initialized")
            .lock()
            .expect("Failed to acquire lock");

        cron.push((key.to_string(), expiry));
    }
}
