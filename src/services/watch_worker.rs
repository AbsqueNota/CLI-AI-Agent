use std::{
    future::Future,
    time::Duration,
};

use tokio::{
    sync::watch,
    task::JoinHandle,
};

pub struct WatchWorker<T> {
    tx: watch::Sender<Option<T>>,
    handle: JoinHandle<()>,
}

impl<T> WatchWorker<T>
where
    T: Clone + Send + Sync + 'static,
{
    pub fn spawn<F, Fut>(handler: F) -> Self
    where
        F: FnMut(T) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send,
    {
        Self::spawn_with_debounce(Duration::ZERO, handler)
    }

    pub fn spawn_with_debounce<F, Fut>(
        debounce: Duration,
        mut handler: F,
    ) -> Self
    where
        F: FnMut(T) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send,
    {
        let (tx, mut rx) = watch::channel::<Option<T>>(None);

        let handle = tokio::spawn(async move {
            while rx.changed().await.is_ok() {
                if !debounce.is_zero() {
                    tokio::time::sleep(debounce).await;
                }

                let latest = rx.borrow_and_update().clone();

                if let Some(payload) = latest {
                    handler(payload).await;
                }
            }
        });

        Self { tx, handle }
    }

    pub fn dispatch(&self, payload: T) {
        let _ = self.tx.send(Some(payload));
    }

    pub async fn shutdown(self) {
        drop(self.tx);
        let _ = self.handle.await;
    }
}