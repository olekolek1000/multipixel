use crate::{
	packet_server,
	server::Server,
	session::{SessionHandle, SessionInstanceWeak},
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

	pub async fn broadcast(&self, packet: &packet_server::Packet, except: Option<&SessionHandle>) {
		for cell in &self.sessions {
			if let Some(except) = except {
				if cell.handle == *except {
					continue;
				}
			}
			if let Some(session_mtx) = cell.instance_mtx.upgrade() {
				let mut session = session_mtx.lock().await;
				session.queue_send_packet(packet.clone());
			}
		}
	}
}

pub type RoomInstanceMutex = Arc<Mutex<RoomInstance>>;
