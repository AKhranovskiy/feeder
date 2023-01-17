use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::signal;

#[derive(Clone)]
pub struct Terminator {
    is_terminated: Arc<AtomicBool>,
}

impl Terminator {
    pub fn new() -> Self {
        Self {
            is_terminated: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn terminate(&self) {
        self.is_terminated.store(true, Ordering::Relaxed);
    }

    pub fn is_terminated(&self) -> bool {
        self.is_terminated.load(Ordering::Relaxed)
    }

    pub async fn signal(&self) {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        tokio::select! {
            _ = ctrl_c => { self.terminate(); },
            _ = terminate => { self.terminate(); },
        }

        println!("signal received, starting graceful shutdown");
    }
}
