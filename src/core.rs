use std::sync::mpsc;
use std::thread;

pub type JobResult = anyhow::Result<String>;

pub struct JobRunner {
    tx: mpsc::Sender<JobResult>,
    rx: mpsc::Receiver<JobResult>,
}

impl JobRunner {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { tx, rx }
    }

    pub fn spawn<F>(&self, f: F)
    where
        F: FnOnce() -> JobResult + Send + 'static,
    {
        let tx = self.tx.clone();
        thread::spawn(move || {
            let _ = tx.send(f());
        });
    }

    pub fn drain(&self) -> Vec<JobResult> {
        let mut out = Vec::new();
        while let Ok(item) = self.rx.try_recv() {
            out.push(item);
        }
        out
    }

    pub fn sender(&self) -> mpsc::Sender<JobResult> {
        self.tx.clone()
    }
}
