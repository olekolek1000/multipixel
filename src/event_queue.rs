#![allow(dead_code)]

use std::{
	collections::VecDeque,
	sync::{Arc, Mutex as SyncMutex},
};

use tokio::sync::{broadcast, Notify};

struct Data<DataType> {
	queue: VecDeque<DataType>,
	pub notifier: Arc<Notify>,
}

#[derive(Clone)]
pub struct EventQueue<DataType> {
	data: Arc<SyncMutex<Data<DataType>>>,
}

impl<DataType> EventQueue<DataType> {
	pub fn new(notifier: Arc<Notify>) -> Self {
		Self {
			data: Arc::new(SyncMutex::new(Data {
				notifier,
				queue: Default::default(),
			})),
		}
	}

	pub fn send(&self, message: DataType) {
		let mut data = self.data.lock().unwrap();
		data.queue.push_back(message);
		data.notifier.notify_waiters();
	}

	pub fn read(&self) -> Option<DataType> {
		let mut data = self.data.lock().unwrap();
		data.queue.pop_front()
	}

	pub fn read_all(&self) -> Vec<DataType> {
		let mut data = self.data.lock().unwrap();
		data.queue.drain(..).collect()
	}
}

#[derive(Clone)]
pub struct NotifySender<T> {
	sender: broadcast::Sender<T>,
	notify: Arc<Notify>,
}

impl<T> NotifySender<T>
where
	T: Clone,
{
	pub fn new(notify: Arc<Notify>, capacity: usize) -> Self {
		let (sender, _) = broadcast::channel::<T>(capacity);
		Self { sender, notify }
	}

	pub fn send(&self, data: T) {
		if let Err(e) = self.sender.send(data) {
			log::error!("sender error: {e}");
		}
		self.notify.notify_one();
	}

	pub fn subscribe(&self) -> broadcast::Receiver<T> {
		self.sender.subscribe()
	}
}
