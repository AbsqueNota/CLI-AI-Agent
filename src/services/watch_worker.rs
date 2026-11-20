use std::future::Future;
use tokio::sync::watch;
use tokio::task::JoinHandle;

/// A universal background worker that processes the latest current state of `T`.
pub struct WatchWorker<T> {
    tx: watch::Sender<Option<T>>,
    handle: JoinHandle<()>,
}

impl<T> WatchWorker<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Launches a background task that reacts to each new value in the channel.
    pub fn spawn<F, Fut>(mut handler: F) -> Self
    where
        F: FnMut(T) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send,
    {
        let (tx, mut rx) = watch::channel::<Option<T>>(None);

        let handle = tokio::spawn(async move {
            while rx.changed().await.is_ok() {
                // Retrieve the latest value and reset the change flag
                let latest = rx.borrow_and_update().clone();

                if let Some(payload) = latest {
                    handler(payload).await;
                }
            }
        });

        Self { tx, handle }
    }

    /// Non-blocking dispatch of the new state to a worker.
    pub fn dispatch(&self, payload: T) {
        let _ = self.tx.send(Some(payload));
    }

    /// Graceful shutdown: closes the channel and waits for the current background task to complete.
    pub async fn shutdown(self) {
        drop(self.tx);
        let _ = self.handle.await;
    }
}