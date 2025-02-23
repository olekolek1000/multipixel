use crate::{
	chunk::system::{ChunkSystem, ChunkSystemMutex, ChunkSystemSignal},
	config::Config,
	database::Database,
	event_queue::{EventQueue, NotifySender},
	packet_server,
	preview_system::{PreviewSystem, PreviewSystemMutex},
	server::Server,
	session::{SessionHandle, SessionInstanceWeak},
	tool::brush::BrushShapes,
};
use std::sync::Arc;
use std::sync::Mutex as SyncMutex;
use tokio::sync::{Mutex, Notify};

#[derive(Clone)]
pub struct RoomSessionData {
	pub instance_mtx: SessionInstanceWeak,
	pub queue_send: EventQueue<packet_server::Packet>,
	pub handle: SessionHandle,
	pub nick_name: Arc<SyncMutex<String>>,
}

pub struct RoomRefs {
	pub room_mtx: RoomInstanceMutex,
	pub chunk_system_mtx: ChunkSystemMutex,
	pub preview_system_mtx: PreviewSystemMutex,
	pub brush_shapes_mtx: Arc<Mutex<BrushShapes>>,
	pub chunk_system_sender: NotifySender<ChunkSystemSignal>,
}

pub struct RoomInstance {
	sessions: Vec<RoomSessionData>,
	pub database: Arc<Mutex<Database>>,
	pub chunk_system: ChunkSystemMutex,
	pub chunk_system_sender: NotifySender<ChunkSystemSignal>,
	pub brush_shapes: Arc<Mutex<BrushShapes>>,
	pub preview_system: PreviewSystemMutex,
	cleaned_up: bool,
}

impl RoomInstance {
	pub async fn new(room_name: &str, config: &Config) -> anyhow::Result<Self> {
		let db_path = format!("rooms/{}.db", room_name);

		let database = Arc::new(Mutex::new(Database::new(db_path.as_str()).await?));
		let preview_system_mtx = Arc::new(Mutex::new(PreviewSystem::new(database.clone())));

		let chunk_system_notifier = Arc::new(Notify::new());
		let chunk_system_sender =
			NotifySender::<ChunkSystemSignal>::new(chunk_system_notifier.clone(), 16);

		let chunk_system_mtx = Arc::new(Mutex::new(ChunkSystem::new(
			database.clone(),
			chunk_system_notifier,
			chunk_system_sender.clone(),
			preview_system_mtx.clone(),
			config.autosave_interval_ms,
		)));

		{
			let mut chunk_system = chunk_system_mtx.lock().await;
			// TODO (low priority): make this system completely redundant (tick-less), improving idle power usage
			ChunkSystem::launch_task_tick(&mut chunk_system, Arc::downgrade(&chunk_system_mtx));
			ChunkSystem::launch_task_processor(&mut chunk_system, Arc::downgrade(&chunk_system_mtx));
		}

		Ok(Self {
			sessions: Vec::new(),
			database: database.clone(),
			cleaned_up: false,
			chunk_system: chunk_system_mtx,
			chunk_system_sender,
			preview_system: preview_system_mtx,
			brush_shapes: Arc::new(Mutex::new(BrushShapes::new())),
		})
	}

	pub fn add_session(
		&mut self,
		server: &Server,
		queue_send: EventQueue<packet_server::Packet>,
		session_handle: &SessionHandle,
		nick_name: Arc<SyncMutex<String>>,
	) {
		log::info!("Adding session ID {}", session_handle.id());
		let session = server.sessions.get(session_handle).unwrap();

		self.sessions.push(RoomSessionData {
			handle: *session_handle,
			queue_send,
			instance_mtx: Arc::downgrade(&session.session),
			nick_name,
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

	pub fn get_all_sessions(&self, except: Option<&SessionHandle>) -> Vec<RoomSessionData> {
		let mut ret: Vec<RoomSessionData> = Vec::new();

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
			for session_data in &self.sessions {
				if session_data.handle == *except {
					continue;
				}

				if *session_data.nick_name.lock().unwrap()
					== Self::gen_suitable_name(current, &occupied_num)
				{
					occupied = true;
					if let Some(num) = &mut occupied_num {
						*num += 1;
					} else {
						occupied_num = Some(2);
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
		log::trace!("Cleaning-up room");
		ChunkSystem::cleanup(self.chunk_system.clone()).await;
		PreviewSystem::process_all(self.preview_system.clone()).await;
		self.database.lock().await.cleanup().await;
		self.cleaned_up = true;
		log::debug!("Room cleaned up");
	}
}

impl Drop for RoomInstance {
	fn drop(&mut self) {
		assert!(self.cleaned_up, "cleanup() not called");
		log::debug!("Room dropped");
	}
}

pub type RoomInstanceMutex = Arc<Mutex<RoomInstance>>;
