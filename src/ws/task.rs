//! Helpers for owning spawned Tokio tasks.

use tokio::task::JoinHandle;

/// Owns a [`JoinHandle`] and aborts it on drop.
///
/// This is used for reconnection handlers that hold strong `Arc` references to their
/// subscription managers. Without an owned abort handle, those tasks can keep the entire
/// WebSocket channel graph alive after the client has been dropped.
pub(crate) struct AbortOnDrop(JoinHandle<()>);

impl AbortOnDrop {
    /// Wrap a spawned task handle so it is aborted when this value drops.
    #[must_use]
    pub(crate) fn new(handle: JoinHandle<()>) -> Self {
        Self(handle)
    }
}

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use super::AbortOnDrop;

    #[tokio::test]
    async fn abort_on_drop_cancels_pending_task() {
        let finished = Arc::new(AtomicBool::new(false));
        let finished_task = Arc::clone(&finished);

        let handle = tokio::spawn(async move {
            std::future::pending::<()>().await;
            finished_task.store(true, Ordering::SeqCst);
        });

        let wrapper = AbortOnDrop::new(handle);
        drop(wrapper);

        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(!finished.load(Ordering::SeqCst));
    }
}
