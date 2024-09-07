use crate::{
	chunk_system::ChunkSystem,
	database::Database,
	event_queue::EventQueue,
	packet_server,
	preview_system::PreviewSystem,
	server::Server,
	session::{SessionHandle, SessionInstanceWeak},
	tool::brush::BrushShapes,
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct SessionCell {
	pub instance_mtx: SessionInstanceWeak,
	pub queue_send: EventQueue<packet_server::Packet>,
	pub handle: SessionHandle,
}

pub struct RoomInstance {
	sessions: Vec<SessionCell>,
	pub database: Arc<Mutex<Database>>,
	pub chunk_system: Arc<Mutex<ChunkSystem>>,
	pub brush_shapes: Arc<Mutex<BrushShapes>>,
	pub preview_system: Arc<Mutex<PreviewSystem>>,
	cleaned_up: bool,
}

impl RoomInstance {
	pub async fn new(room_name: &str) -> anyhow::Result<Self> {
		let db_path = format!("rooms/{}.db", room_name);

		let database = Arc::new(Mutex::new(Database::new(db_path.as_str()).await?));

		Ok(Self {
			sessions: Vec::new(),
			database: database.clone(),
			cleaned_up: false,
			chunk_system: Arc::new(Mutex::new(ChunkSystem::new(database.clone()))),
			preview_system: Arc::new(Mutex::new(PreviewSystem::new(database.clone()))),
			brush_shapes: Arc::new(Mutex::new(BrushShapes::new())),
		})
	}

	pub fn add_session(
		&mut self,
		server: &Server,
		queue_send: EventQueue<packet_server::Packet>,
		session_handle: &SessionHandle,
	) {
		log::info!("Adding session ID {}", session_handle.id());
		let session = server.sessions.get(session_handle).unwrap();

		self.sessions.push(SessionCell {
			handle: *session_handle,
			queue_send,
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

	pub fn broadcast(&self, packet: &packet_server::Packet, except: Option<&SessionHandle>) {
		for cell in &self.sessions {
			if let Some(except) = except {
				if cell.handle == *except {
					continue;
				}
			}
			cell.queue_send.send(packet.clone());
		}
	}

	pub fn get_all_sessions(&self, except: Option<&SessionHandle>) -> Vec<SessionCell> {
		let mut ret: Vec<SessionCell> = Vec::new();

		for cell in &self.sessions {
			if let Some(except) = except {
				if cell.handle == *except {
					continue;
				}
			}
			ret.push(cell.clone());
		}

		ret
	}

	fn gen_suitable_name(current: &str, occupied_num: &Option<u32>) -> String {
		if let Some(num) = occupied_num {
			format!("{} ({})", current, num)
		} else {
			String::from(current)
		}
	}

	pub async fn get_suitable_nick_name(&self, current: &str, except: &SessionHandle) -> String {
		let mut occupied_num: Option<u32> = None;

		loop {
			let mut occupied = false;
			for session in &self.sessions {
				if session.handle == *except {
					continue;
				}

				if let Some(session) = session.instance_mtx.upgrade() {
					if session.lock().await.nick_name == Self::gen_suitable_name(current, &occupied_num) {
						occupied = true;
						if let Some(num) = &mut occupied_num {
							*num += 1;
						} else {
							occupied_num = Some(2);
						}
					}
				}
			}

			if !occupied {
				break;
			}
		}

		Self::gen_suitable_name(current, &occupied_num)
	}

	pub async fn cleanup(&mut self) {
		self.database.lock().await.cleanup().await;
		self.cleaned_up = true;
	}
}

impl Drop for RoomInstance {
	fn drop(&mut self) {
		assert!(self.cleaned_up, "cleanup() not called");
	}
}

pub type RoomInstanceMutex = Arc<Mutex<RoomInstance>>;
