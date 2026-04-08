#[derive(Debug, Clone)]
pub struct Snapshot {
    pub cpu: CpuSnapshot,
}

#[derive(Debug, Clone)]
pub struct CpuSnapshot {
    pub total_samples: u64,
    pub frames: Vec<HotFrame>,
}

#[derive(Debug, Clone)]
pub struct HotFrame {
    pub count: u64,
    pub percent: f64,
    pub addr: u64,
    pub symbol: String,
}
