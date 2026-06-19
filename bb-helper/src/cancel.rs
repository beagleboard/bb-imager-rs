use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Clone)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    pub fn drop_guard(&self) -> DropGuard {
        DropGuard(self.0.clone())
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
}

pub struct DropGuard(Arc<AtomicBool>);

impl Drop for DropGuard {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed)
    }
}
