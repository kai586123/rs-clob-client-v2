//! Helpers for owning spawned Tokio tasks.

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Owns a [`JoinHandle`] and aborts it on drop.
///
/// This is used for reconnection handlers that hold strong `Arc` references to their
/// subscription managers. Without an owned abort handle, those tasks can keep the entire
/// WebSocket channel graph alive after the client has been dropped.
pub(crate) struct AbortOnDrop {
    handle: JoinHandle<()>,
    cancel: Option<CancellationToken>,
}

impl AbortOnDrop {
    /// Wrap a spawned task handle so it is aborted when this value drops.
    #[must_use]
    #[allow(
        dead_code,
        reason = "Useful for task handles that do not own a connection token"
    )]
    pub(crate) fn new(handle: JoinHandle<()>) -> Self {
        Self {
            handle,
            cancel: None,
        }
    }

    /// Wrap a spawned task handle and cancel the provided token before aborting the task.
    #[must_use]
    pub(crate) fn with_cancel(handle: JoinHandle<()>, cancel: CancellationToken) -> Self {
        Self {
            handle,
            cancel: Some(cancel),
        }
    }
}

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        if let Some(cancel) = &self.cancel {
            cancel.cancel();
        }
        self.handle.abort();
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
