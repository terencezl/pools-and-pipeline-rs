use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct InterruptIndicator {
    state: Arc<AtomicBool>,
}

impl InterruptIndicator {
    pub fn new() -> Self {
        let state = Arc::new(AtomicBool::new(false));
        let state_ = state.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install CTRL+C handler");
            state_.store(true, Ordering::Relaxed);
        });
        Self { state }
    }

    pub fn is_set(&self) -> bool {
        self.state.load(Ordering::Relaxed)
    }
}
