use crate::{
	gen_id,
	server::{Server, ServerMutex},
	session::{SessionHandle, SessionInstanceMutex, SessionInstanceWeak},
};
use std::sync::Arc;
use tokio::sync::Mutex;

struct SessionCell {
	instance_mtx: SessionInstanceWeak,
	handle: SessionHandle,
}

pub struct RoomInstance {
	sessions: Vec<SessionCell>,
}

impl RoomInstance {
	pub fn new() -> Self {
		Self {
			sessions: Vec::new(),
		}
	}

	pub fn add_session(&mut self, server: &Server, session_handle: &SessionHandle) {
		log::info!("Adding session ID {}", session_handle.id());
		let session = server.sessions.get(session_handle).unwrap();

		self.sessions.push(SessionCell {
			handle: *session_handle,
			instance_mtx: Arc::downgrade(session),
		});
	}

	pub fn remove_session(&mut self, session_handle: &SessionHandle) {
		log::info!("Removing session ID {}", session_handle.id());
		self.sessions.retain(|cell| cell.handle != *session_handle);
	}

	pub fn wants_to_be_removed(&self) -> bool {
		self.sessions.is_empty()
	}
}

pub type RoomInstanceMutex = Arc<Mutex<RoomInstance>>;
