#![allow(dead_code)]

use std::sync::Arc;

use glam::IVec2;
use tokio::sync::Mutex;

use crate::database::{Database, DatabaseFunc};

pub struct PreviewSystemLayer {
	zoom: u8,
	update_queue: Vec<IVec2>,
}

impl PreviewSystemLayer {
	pub fn new(zoom: u8) -> Self {
		Self {
			zoom,
			update_queue: Vec::new(),
		}
	}

	pub fn add_to_queue(&mut self, coords: IVec2) {
		// Check if already added to queue (reverse iterator)
		for it in self.update_queue.iter().rev() {
			if *it == coords {
				return; // Already added to the queue
			}
		}

		self.update_queue.push(coords);
	}

	pub fn process_one_block(&mut self) {
		if self.update_queue.is_empty() {
			return; // Nothing to do
		}

		//let position = self.update_queue.pop();
		todo!()
	}
}

pub struct PreviewSystem {
	//layers: Vec<PreviewSystemLayer>,
	database: Arc<Mutex<Database>>,
	update_queue_cache: Vec<IVec2>,
}

impl PreviewSystem {
	pub fn new(database: Arc<Mutex<Database>>) -> Self {
		Self {
			database,
			update_queue_cache: Vec::new(),
		}
	}

	fn layer_index_to_zoom(index: u8) -> u8 {
		index + 1
	}

	pub async fn request_data(&self, pos: &IVec2, zoom: u8) -> anyhow::Result<Option<Arc<Vec<u8>>>> {
		if let Some(record) = DatabaseFunc::preview_load_data(&self.database, *pos, zoom).await? {
			Ok(Some(record.data))
		} else {
			Ok(None)
		}
	}

	pub fn add_to_queue_front(&mut self, coords: IVec2) {
		self.update_queue_cache.push(coords);
	}

	pub fn get_layer_count(&self) -> u8 {
		// const for now
		5
	}
}

pub type PreviewSystemMutex = Arc<Mutex<PreviewSystem>>;
