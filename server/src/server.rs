use crate::session::{SessionHandle, SessionInstance, SessionVec};

pub struct Server {
	pub sessions: SessionVec,
}

impl Server {
	pub fn new() -> Self {
		Self {
			sessions: SessionVec::new(),
		}
	}

	pub fn create_session(&mut self) -> SessionHandle {
		let session = SessionInstance::new();
		self.sessions.add(session)
	}
}
