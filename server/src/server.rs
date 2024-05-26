use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

use crate::{
	room::{RoomHandle, RoomInstance, RoomInstanceMutex, RoomVec},
	session::{SessionHandle, SessionInstance, SessionVec},
};

pub struct Server {
	pub sessions: SessionVec,
	pub rooms: RoomVec,
	pub rooms_by_name: HashMap<String, RoomHandle>,
}

pub type ServerMutex = Arc<Mutex<Server>>;

impl Server {
	pub fn new() -> Self {
		Self {
			sessions: SessionVec::new(),
			rooms: RoomVec::new(),
			rooms_by_name: HashMap::new(),
		}
	}

	pub fn create_session(&mut self) -> SessionHandle {
		let session = SessionInstance::new();
		self.sessions.add(Arc::new(Mutex::new(session)))
	}

	pub async fn get_or_load_room(&mut self, name: &str) -> (RoomHandle, RoomInstanceMutex) {
		if let Some(handle) = self.rooms_by_name.get(name) {
			//Get existing room
			if let Some(room) = self.rooms.get(handle) {
				return (*handle, room.clone());
			} else {
				unreachable!();
			}
		}

		let room = Arc::new(Mutex::new(RoomInstance::new()));
		(self.rooms.add(room.clone()), room)
	}
}
