use std::sync::atomic::{AtomicU64, Ordering};

pub struct IdGenerator {
    counter: AtomicU64,
}

impl IdGenerator {
    pub fn new(start: u64) -> Self {
        Self {
            counter: AtomicU64::new(start),
        }
    }

    pub fn next(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }
}
