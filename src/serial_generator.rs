use parking_lot::Mutex as SyncMutex;
use std::sync::Arc;

pub struct SerialGenerator {
	serial: Arc<SyncMutex<u64>>,
}

impl SerialGenerator {
	pub fn new() -> Self {
		Self {
			serial: Arc::new(SyncMutex::new(0)),
		}
	}

	pub fn increment_get(&self) -> u64 {
		let mut serial = self.serial.lock();
		let cur = *serial;
		*serial += 1;
		cur
	}
}
