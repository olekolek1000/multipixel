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
		let session = SessionInstance::new();
		let session_mtx = Arc::new(Mutex::new(session));
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

	pub fn cleanup_rooms(&mut self) {
		tokio::task::block_in_place(move || {
			self.rooms.retain(|room_name, room| {
				if room.blocking_lock().wants_to_be_removed() {
					log::info!("Freeing room with name {}", room_name);
					return false;
				}
				true
			})
		})
	}

	pub async fn remove_session(&mut self, session_handle: &SessionHandle) {
		log::info!("Removing session ID {}", session_handle.id());
		self.sessions.remove(session_handle);
		self.cleanup_rooms();
	}
}
