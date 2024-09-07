#![allow(dead_code)]

use std::sync::Arc;

use glam::IVec2;
use tokio::sync::Mutex;

use crate::{database::Database, room::RoomInstance};

pub struct PreviewSystemLayer {
	// not used for now
}

pub struct PreviewSystem {
	//layers: Vec<PreviewSystemLayer>,
	database: Arc<Mutex<Database>>,
}

impl PreviewSystem {
	pub fn new(database: Arc<Mutex<Database>>) -> Self {
		Self { database }
	}

	fn layer_index_to_zoom(index: u8) -> u8 {
		index + 1
	}

	pub async fn request_data(&self, pos: &IVec2, zoom: u8) -> anyhow::Result<Option<Arc<Vec<u8>>>> {
		let pos = *pos;
		if let Some(record) = self
			.database
			.lock()
			.await
			.client
			.conn(move |conn| Database::preview_load_data(conn, &pos, zoom))
			.await?
		{
			Ok(Some(record.data))
		} else {
			Ok(None)
		}
	}
}

pub type PreviewSystemMutex = Arc<Mutex<PreviewSystem>>;
