use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use crossterm::event::{
    self,
    Event,
    KeyEventKind,
};

use tokio::{
    sync::mpsc,
    task::JoinHandle,
};

use crate::app::events::AppEvent;

pub struct TerminalEventReader {
    stop: Arc<AtomicBool>,
    task: Option<JoinHandle<io::Result<()>>>,
}

impl TerminalEventReader {
    pub fn start(
        tx: mpsc::Sender<AppEvent>,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);

        let task = tokio::task::spawn_blocking(move || {
            while !worker_stop.load(Ordering::Relaxed) {
                if !event::poll(Duration::from_millis(50))? {
                    continue;
                }

                match event::read()? {
                    Event::Key(key) => {
                        if key.kind == KeyEventKind::Release {
                            continue;
                        }

                        if tx.blocking_send(
                            AppEvent::Key(key)
                        ).is_err() {
                            break;
                        }
                    }

                    Event::Resize(width, height) => {
                        if tx.blocking_send(
                            AppEvent::Resize {
                                width,
                                height,
                            }
                        ).is_err() {
                            break;
                        }
                    }

                    _ => {}
                }
            }

            Ok(())
        });

        Self {
            stop,
            task: Some(task),
        }
    }

    pub async fn shutdown(
        mut self,
    ) -> io::Result<()> {
        self.stop.store(true, Ordering::Relaxed);

        let Some(task) = self.task.take() else {
            return Ok(());
        };

        match task.await {
            Ok(result) => result,
            Err(error) => Err(io::Error::other(error)),
        }
    }
}

impl Drop for TerminalEventReader {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}