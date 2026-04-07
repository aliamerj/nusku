use std::collections::HashMap;

/// Counts how many times each function appears across all samples in a window.
pub struct Aggregator {
    counts: HashMap<String, u64>,
    total: u64,
}

impl Aggregator {
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
            total: 0,
        }
    }

    pub fn reset(&mut self) {
        self.counts.clear();
        self.total = 0;
    }
}
