use std::sync::{Arc, Mutex as SyncMutex};

pub struct SerialGenerator {
	serial: Arc<SyncMutex<u64>>,
}

impl SerialGenerator {
	pub fn new() -> SerialGenerator {
		Self {
			serial: Arc::new(SyncMutex::new(0)),
		}
	}

	pub fn increment_get(&self) -> u64 {
		let mut serial = self.serial.lock().unwrap();
		let cur = *serial;
		*serial += 1;
		cur
	}
}
