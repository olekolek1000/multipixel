use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

use crate::{
	room::{RoomInstance, RoomInstanceMutex},
	session::{SessionHandle, SessionInstance, SessionInstanceMutex, SessionVec},
};

pub struct Server {
	pub sessions: SessionVec,
	pub rooms: HashMap<String /* Room name */, RoomInstanceMutex>,
}

pub type ServerMutex = Arc<Mutex<Server>>;

impl Server {
	pub fn new() -> ServerMutex {
		Arc::new(Mutex::new(Self {
			sessions: SessionVec::new(),
			rooms: HashMap::new(),
		}))
	}

	pub fn create_session(&mut self) -> (SessionHandle, SessionInstanceMutex) {
		let session_mtx = Arc::new(Mutex::new(SessionInstance::new()));
		(self.sessions.add(session_mtx.clone()), session_mtx)
	}

	pub async fn get_or_load_room(&mut self, room_name: &str) -> RoomInstanceMutex {
		if let Some(room_mtx) = self.rooms.get(room_name) {
			//Get existing room
			return room_mtx.clone();
		}

		log::info!("Creating room with name {}", room_name);
		let room = Arc::new(Mutex::new(RoomInstance::new()));
		self.rooms.insert(String::from(room_name), room.clone());
		room
	}

	pub async fn add_session_to_room(
		&mut self,
		room_name: &str,
		session_handle: &SessionHandle,
	) -> RoomInstanceMutex {
		let room_instance_mtx = self.get_or_load_room(room_name).await;

		room_instance_mtx
			.lock()
			.await
			.add_session(self, session_handle);

		room_instance_mtx
	}

	pub async fn cleanup_rooms(&mut self) {
		// .retain() cannot be used in async environment
		let mut rooms_to_remove: Vec<String> = Vec::new();
		for (room_name, room) in &self.rooms {
			if room.lock().await.wants_to_be_removed() {
				rooms_to_remove.push(room_name.clone());
			}
		}

		for room_name in rooms_to_remove {
			log::info!("Freeing room with name {}", room_name);
			self.rooms.remove(room_name.as_str());
		}
	}

	pub async fn remove_session(&mut self, session_handle: &SessionHandle) {
		log::info!("Removing session ID {}", session_handle.id());
		self.sessions.remove(session_handle);
		self.cleanup_rooms().await;
	}
}
