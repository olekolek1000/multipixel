#![allow(dead_code)]

use std::sync::{Arc, Mutex as SyncMutex};

use tokio::sync::Notify;

struct Data {
	pub notifier: Arc<Notify>,
	triggered: bool,
}

// note to myself: this struct is probably just a skill issue
// and i'm almost 100% sure there is a better implementation somewhere.
#[derive(Clone)]
pub struct Signal {
	data: Arc<SyncMutex<Data>>,
}

impl Signal {
	pub fn new(notifier: Arc<Notify>) -> Self {
		Self {
			data: Arc::new(SyncMutex::new(Data {
				notifier,
				triggered: false,
			})),
		}
	}

	pub fn notify(&self) {
		let mut data = self.data.lock().unwrap();
		data.triggered = true;
		data.notifier.notify_waiters();
	}

	pub fn check_triggered(&self) -> bool {
		let mut data = self.data.lock().unwrap();
		if data.triggered {
			data.triggered = false;
			true
		} else {
			false
		}
	}
}
