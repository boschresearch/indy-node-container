use crate::worker::worker::IndyWorker;
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures_executor::block_on;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use futures::SinkExt;

pub enum Message {
    Exit,
}

pub type CloseReceiver = Receiver<Message>;
pub type CloseSender = Sender<Message>;

pub struct ThreadedWorker {
    worker: Arc<Mutex<IndyWorker>>,
    handle: Option<JoinHandle<(u64, u64)>>,
    sender: Option<CloseSender>,
}

impl ThreadedWorker {
    pub fn new(
        seed: String,
        genesis_path: String,
        id: String,
        reads: i8,
    ) -> Result<ThreadedWorker, Box<dyn Error>> {
        let worker = IndyWorker::new(seed, genesis_path, id, reads);
        match worker {
            Err(err) => Err(err),
            Ok(worker) => Ok(ThreadedWorker {
                worker: Arc::new(Mutex::new(worker)),
                handle: None,
                sender: None,
            }),
        }
    }

    pub fn start(&mut self) {
        let local_self = self.worker.clone();
        let (sender, mut receiver) = channel(2);
        let handle = thread::spawn(move || {
            let mut inner = local_self.lock().unwrap();
            let future = inner.run_loop(&mut receiver);
            let result = block_on(future);
            return result;
        });
        self.handle = Some(handle);
        self.sender = Some(sender);
    }

    pub fn stop(&mut self) -> Result<(), Box<dyn Error>> {
        if self.handle.is_none() || self.sender.is_none() {
            Err("Worker was not started, cannot stop".into())
        } else {
            let _ = self.sender.as_mut().unwrap().send(Message::Exit);
            Ok(())
        }
    }

    pub fn get_handle(self) -> Option<JoinHandle<(u64, u64)>> {
        return self.handle;
    }
}
