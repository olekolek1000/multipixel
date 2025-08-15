use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::{
	config,
	event_queue::EventQueue,
	packet_server,
	room::{RoomInstance, RoomInstanceMutex},
	session::{self, SessionHandle, SessionInstance, SessionInstanceMutex, SessionState, SessionVec},
};

use std::sync::Mutex as SyncMutex;

pub struct Server {
	cancel_token: CancellationToken,
	pub sessions: SessionVec,
	pub rooms: HashMap<String /* Room name */, RoomInstanceMutex>,
	pub config: config::Config,
}

pub type ServerMutex = Arc<Mutex<Server>>;

impl Server {
	pub fn new(config: config::Config, cancel_token: CancellationToken) -> ServerMutex {
		Arc::new(Mutex::new(Self {
			cancel_token,
			sessions: SessionVec::new(),
			rooms: HashMap::new(),
			config,
		}))
	}

	pub async fn save_and_exit(&mut self) -> anyhow::Result<()> {
		self.kick_all_sessions().await?;
		self.cleanup_rooms(true).await;

		self.cancel_token.cancel();
		Ok(())
	}

	async fn kick_all_sessions(&mut self) -> anyhow::Result<()> {
		let mut handles = Vec::new();

		for (idx, cell) in self.sessions.vec.iter().enumerate() {
			if let Some(cell) = cell {
				let handle = SessionVec::get_handle(cell, idx);
				log::info!("Cleaning-up session ID {idx}");
				let mut session = cell.obj.session.lock().await;
				session.kick("Server closed");
				// Send remaining data to the client
				let _dont_care = session.send_all().await;
				session.cleanup(&handle).await;
				handles.push(handle);
			}
		}

		for handle in handles {
			self.remove_session(&handle).await;
		}

		Ok(())
	}

	pub fn create_session(
		&mut self,
		cancel_token: CancellationToken,
	) -> (SessionHandle, SessionInstanceMutex) {
		let session_mtx = Arc::new(Mutex::new(SessionInstance::new(cancel_token)));
		(
			self.sessions.add(session::SessionContainer {
				session: session_mtx.clone(),
			}),
			session_mtx,
		)
	}

	pub async fn get_or_load_room(&mut self, room_name: &str) -> anyhow::Result<RoomInstanceMutex> {
		if let Some(room_mtx) = self.rooms.get(room_name) {
			//Get existing room
			return Ok(room_mtx.clone());
		}

		log::info!("Creating room with name {room_name}");
		let room = Arc::new(Mutex::new(
			RoomInstance::new(room_name, &self.config).await?,
		));
		self.rooms.insert(String::from(room_name), room.clone());
		Ok(room)
	}

	pub async fn add_session_to_room(
		&mut self,
		room_name: &str,
		queue_send: EventQueue<packet_server::Packet>,
		session_handle: &SessionHandle,
		state: Arc<SyncMutex<SessionState>>,
	) -> anyhow::Result<RoomInstanceMutex> {
		let room_instance_mtx = self.get_or_load_room(room_name).await?;

		room_instance_mtx
			.lock()
			.await
			.add_session(self, queue_send, session_handle, state);

		Ok(room_instance_mtx)
	}

	pub async fn cleanup_rooms(&mut self, force_all: bool) {
		// .retain() cannot be used in async environment
		let mut rooms_to_remove: Vec<String> = Vec::new();
		for (room_name, room) in &self.rooms {
			let mut room = room.lock().await;
			if room.wants_to_be_removed() || force_all {
				room.cleanup().await;
				rooms_to_remove.push(room_name.clone());
			}
		}

		for room_name in rooms_to_remove {
			log::info!("Freeing room with name {room_name}");
			self.rooms.remove(room_name.as_str());
		}
	}

	pub async fn remove_session(&mut self, session_handle: &SessionHandle) {
		log::info!("Removing session ID {}", session_handle.id());
		self.sessions.remove(session_handle);
		self.cleanup_rooms(false).await;
	}
}
