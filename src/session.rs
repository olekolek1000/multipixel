use binary_reader::BinaryReader;
use futures_util::SinkExt;
use glam::IVec2;
use std::collections::VecDeque;
use std::error::Error;
use std::sync::{Arc, Weak};
use std::time::Duration;
use std::{fmt, io};
use tokio::sync::{Mutex, MutexGuard, Notify};
use tokio_websockets::Message;

use crate::chunk::ChunkInstanceWeak;
use crate::chunk_system::{ChunkSystem, ChunkSystemMutex};
use crate::packet_client::ClientCmd;
use crate::room::{RoomInstance, RoomInstanceMutex};
use crate::server::ServerMutex;
use crate::{gen_id, limits, packet_client, packet_server, util, ConnectionWriter};

// Any protocol usage error that shouldn't be tolerated.
// Causes termination of the session with the kick message provided in the UserError constructor.
#[derive(Debug, Clone)]
struct UserError {
	message: String,
}

impl UserError {
	fn new(message: &str) -> Self {
		Self {
			message: String::from(message),
		}
	}
}

impl Error for UserError {}

impl fmt::Display for UserError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.message)
	}
}

struct ToolData {
	pub size: u8,
	pub color: packet_client::Color,
	pub tool_type: Option<packet_client::ToolType>,
}

impl Default for ToolData {
	fn default() -> Self {
		Self {
			size: 2,
			color: Default::default(),
			tool_type: Default::default(),
		}
	}
}

struct LinkedChunk {
	pos: IVec2,
	chunk: ChunkInstanceWeak,
	outside_boundary_duration: u32,
}

impl LinkedChunk {
	fn new(pos: &IVec2, chunk: &ChunkInstanceWeak) -> Self {
		Self {
			pos: pos.clone(),
			chunk: chunk.clone(),
			outside_boundary_duration: 0,
		}
	}
}

#[derive(Default)]
struct Boundary {
	start_x: i32,
	start_y: i32,
	end_x: i32,
	end_y: i32,
	zoom: f32,
}

pub struct SessionInstance {
	pub nick_name: String, // Max 255 characters

	packets_to_send: VecDeque<packet_server::Packet>,
	notify_send: Arc<Notify>,

	cursor_pos: packet_client::PacketCursorPos,
	cursor_pos_prev: packet_client::PacketCursorPos,
	cursor_pos_sent: Option<packet_client::PacketCursorPos>,
	cursor_down: bool,
	cursor_just_clicked: bool,

	needs_boundary_test: bool,
	boundary: Boundary,

	linked_chunks: Vec<LinkedChunk>,
	chunks_sent: u32,     // Number of chunks received by the client
	chunks_received: u32, // Number of chunks sent by the server

	tool: ToolData,

	room_mtx: Option<RoomInstanceMutex>,
	chunk_system_mtx: Option<ChunkSystemMutex>,

	kicked: bool,
	announced: bool,
	cleaned_up: bool,
}

impl SessionInstance {
	pub fn new() -> Self {
		Self {
			nick_name: String::new(),
			cursor_pos: Default::default(),
			cursor_pos_prev: Default::default(),
			cursor_pos_sent: None,
			cursor_down: false,
			cursor_just_clicked: false,
			kicked: false,
			announced: false,
			needs_boundary_test: false,
			tool: Default::default(),
			room_mtx: Default::default(),
			chunk_system_mtx: Default::default(),
			packets_to_send: Default::default(),
			notify_send: Arc::new(Notify::new()),
			chunks_received: 0,
			chunks_sent: 0,
			linked_chunks: Default::default(),
			boundary: Default::default(),
			cleaned_up: false,
		}
	}

	pub fn launch_tick_task(session_handle: SessionHandle, session_weak: SessionInstanceWeak) {
		tokio::task::Builder::new()
			.name(format!("Session {} tick task", session_handle.id()).as_str())
			.spawn(async move {
				let mut ticks: u32 = 0;
				while let Some(session_mtx) = session_weak.upgrade() {
					let mut session = session_mtx.lock().await;
					if session.cleaned_up {
						break;
					}

					session.tick_cursor(&session_handle).await;
					session.tick_boundary_check().await;

					if ticks % 20 == 0 {
						session.tick_chunks_cleanup().await;
					}

					ticks += 1;

					drop(session);

					// TODO (low priority): calculate execution time instead of ticking every 50ms
					tokio::time::sleep(Duration::from_millis(50)).await;
				}

				log::trace!("Session tick task ended");
			})
			.unwrap();
	}

	pub fn launch_sender_task(
		session_id: u32,
		session_weak: SessionInstanceWeak,
		mut writer: ConnectionWriter,
	) {
		tokio::task::Builder::new()
			.name(format!("Session {} sender task", session_id).as_str())
			.spawn(async move {
				while let Some(session_mtx) = session_weak.upgrade() {
					let mut session = session_mtx.lock().await;

					let notify = session.notify_send.clone(); // Wait for incoming send data if requested
					drop(session);
					notify.notified().await;
					session = session_mtx.lock().await;
					if session.cleaned_up {
						break; // End this task
					}

					let _ = session.send_all(&mut writer).await;
				}
				log::trace!("Session sender task ended");
			})
			.unwrap();
	}

	async fn parse_command(
		&mut self,
		command: ClientCmd,
		reader: &mut BinaryReader,
		session_handle: &SessionHandle,
		server_mtx: &ServerMutex,
	) -> anyhow::Result<()> {
		match command {
			ClientCmd::Announce => {
				self
					.process_command_announce(reader, session_handle, server_mtx)
					.await?
			}
			ClientCmd::Message => self.process_command_message(reader).await?,
			ClientCmd::Ping => self.process_command_ping(reader).await,
			ClientCmd::CursorPos => {
				self
					.process_command_cursor_pos(reader, session_handle)
					.await?
			}
			ClientCmd::CursorDown => self.process_command_cursor_down(reader).await?,
			ClientCmd::CursorUp => self.process_command_cursor_up(reader).await?,
			ClientCmd::Boundary => self.process_command_boundary(reader).await,
			ClientCmd::ChunksReceived => self.process_command_chunks_received(reader).await,
			ClientCmd::PreviewRequest => self.process_command_preview_request(reader).await,
			ClientCmd::ToolSize => self.process_command_tool_size(reader).await?,
			ClientCmd::ToolColor => self.process_command_tool_color(reader).await?,
			ClientCmd::ToolType => self.process_command_tool_type(reader).await?,
			ClientCmd::Undo => self.process_command_undo(reader).await,
		}

		Ok(())
	}

	pub async fn process_payload(
		&mut self,
		session_handle: &SessionHandle,
		data: &[u8],
		server_mtx: &ServerMutex,
	) -> anyhow::Result<()> {
		if let Err(e) = self
			.process_payload_wrap(session_handle, data, server_mtx)
			.await
		{
			if e.is::<UserError>() {
				log::error!("User error: {}", e);
				let _ = self.kick(format!("User error: {}", e).as_str()).await;
			} else if e.is::<io::Error>() {
				log::error!("IO error: {}", e);
				let _ = self.kick(format!("IO error: {}", e).as_str()).await;
			} else {
				log::error!("Unknown error: {}", e);
				let _ = self.kick("Internal server error. This is a bug.").await;
				return Err(anyhow::anyhow!("Internal server error: {}", e));
			}
		}

		Ok(())
	}

	pub async fn process_payload_wrap(
		&mut self,
		session_handle: &SessionHandle,
		data: &[u8],
		server_mtx: &ServerMutex,
	) -> anyhow::Result<()> {
		let mut reader = BinaryReader::from_u8(data);
		reader.set_endian(binary_reader::Endian::Big);

		let command = ClientCmd::try_from(reader.read_u16()?)?;

		//log::trace!("Session ID: {}, command {:?}", session_id, command);
		self
			.parse_command(command, &mut reader, session_handle, server_mtx)
			.await?;

		Ok(())
	}

	async fn history_create_snapshot(&mut self) {
		log::warn!("history_create_snapshot TODO");
	}

	async fn update_cursor(&mut self) {
		if let Some(tool_type) = &self.tool.tool_type {
			match tool_type {
				packet_client::ToolType::Brush => self.update_cursor_brush().await,
				packet_client::ToolType::Fill => self.update_cursor_fill().await,
			}
		}

		self.cursor_just_clicked = false;
	}

	async fn update_cursor_brush(&mut self) {
		if !self.cursor_down {
			return;
		}
		log::warn!("update_cursor_brush TODO");
	}

	async fn update_cursor_fill(&mut self) {
		log::warn!("update_cursor_fill TODO");
	}

	pub fn queue_send_packet(&mut self, packet: packet_server::Packet) {
		self.packets_to_send.push_back(packet);
		//log::trace!("Informed");
		self.notify_send.notify_waiters();
	}

	pub async fn send_all(&mut self, writer: &mut ConnectionWriter) -> anyhow::Result<()> {
		//log::trace!("Sending {} packets", self.packets_to_send.len());
		while let Some(packet) = self.packets_to_send.pop_front() {
			writer.send(Message::binary(packet.data)).await?;
		}
		Ok(())
	}

	async fn kick(&mut self, cause: &str) -> Result<(), tokio_websockets::Error> {
		if self.kicked {
			//Enough
			return Ok(());
		}
		self.queue_send_packet(packet_server::prepare_packet_kick(cause));
		self.kicked = true;
		Ok(())
	}

	pub async fn cleanup(&mut self, session_handle: &SessionHandle) {
		// only this for now
		self.leave_room(session_handle).await;

		self.cleaned_up = true;

		// Inform sender task to exit itself
		self.notify_send.notify_one();
	}

	pub async fn leave_room(&mut self, session_handle: &SessionHandle) {
		if let Some(room_mtx) = &self.room_mtx {
			let mut room = room_mtx.lock().await;

			// Remove itself from the room
			room.remove_session(session_handle);

			// Announce to all other sessions that our session is leaving this room
			let other_sessions = room.get_all_sessions(None).await;
			drop(room);

			for other_session in other_sessions {
				if let Some(session_mtx) = other_session.instance_mtx.upgrade() {
					session_mtx
						.lock()
						.await
						.queue_send_packet(packet_server::prepare_packet_user_remove(
							session_handle.id(),
						));
				}
			}
		}
	}

	async fn join_room(
		&mut self,
		room_name: &str,
		session_handle: &SessionHandle,
		server_mtx: &ServerMutex,
	) -> anyhow::Result<RoomInstanceMutex> {
		let mut server = server_mtx.lock().await;

		// Fetch room and chunk system references
		let room_mtx = server
			.add_session_to_room(room_name, session_handle)
			.await?;
		self.room_mtx = Some(room_mtx.clone());

		let room = room_mtx.lock().await;
		self.chunk_system_mtx = Some(room.chunk_system.clone());
		drop(room);

		Ok(room_mtx)
	}

	async fn process_command_announce(
		&mut self,
		reader: &mut BinaryReader,
		session_handle: &SessionHandle,
		server_mtx: &ServerMutex,
	) -> anyhow::Result<()> {
		if self.announced {
			return Err(UserError::new("Announced more than once"))?;
		}
		self.announced = true;

		let packet = packet_client::PacketAnnounce::read(reader)?;
		log::info!(
			"Session announced as {}, requested room name {}",
			packet.nick_name,
			packet.room_name
		);

		if packet.room_name.len() < limits::ROOM_NAME_LEN_MIN as usize
			|| packet.room_name.len() > limits::ROOM_NAME_LEN_MAX as usize
		{
			return Err(UserError::new(
				format!(
					"Invalid room name length (min {}, max {})",
					limits::ROOM_NAME_LEN_MIN,
					limits::ROOM_NAME_LEN_MAX
				)
				.as_str(),
			))?;
		}

		for ch in packet.room_name.chars() {
			if !ch.is_ascii_alphanumeric() {
				return Err(UserError::new(
					format!(
						"Room name contains invalid character: \"{}\". Only alphanumeric characters are allowed.",
						ch
					)
					.as_str(),
				))?;
			}
		}

		if packet.nick_name.len() < limits::NICK_NAME_LEN_MIN as usize
			|| packet.nick_name.len() > limits::NICK_NAME_LEN_MAX as usize
		{
			return Err(UserError::new(
				format!(
					"Invalid nick name length (min {}, max {})",
					limits::NICK_NAME_LEN_MIN,
					limits::NICK_NAME_LEN_MAX
				)
				.as_str(),
			))?;
		}

		for ch in packet.nick_name.chars() {
			if !ch.is_alphanumeric() && ch != '_' && ch != '-' {
				return Err(UserError::new(
					format!(
						"Nick name contains invalid character: \"{}\". Only alphanumeric characters are allowed.",
						ch
					)
					.as_str(),
				))?;
			}
		}

		self.queue_send_packet(packet_server::prepare_packet_your_id(session_handle.id()));

		// Load room
		let room_mtx = self
			.join_room(&packet.room_name, session_handle, server_mtx)
			.await?;

		self.nick_name = room_mtx
			.lock()
			.await
			.get_suitable_nick_name(packet.nick_name.as_str(), session_handle)
			.await;

		// Broadcast to all users that this user is available
		self.broadcast_self(room_mtx, session_handle).await;

		// Reset tool state
		self.tool = ToolData::default();

		Ok(())
	}

	async fn broadcast_self(&mut self, room_mtx: RoomInstanceMutex, session_handle: &SessionHandle) {
		let room = room_mtx.lock().await;

		// Announce itself to other existing sessions
		room
			.broadcast(
				&packet_server::prepare_packet_user_create(session_handle.id(), &self.nick_name),
				Some(session_handle),
			)
			.await;

		// Send annountement packets to this session from all other existing sessions (its positions and states)
		let other_sessions = room.get_all_sessions(Some(session_handle)).await;

		drop(room); //No more needed in this context

		for other_session in other_sessions {
			if let Some(session_mtx) = other_session.instance_mtx.upgrade() {
				let session = session_mtx.lock().await;
				let other_session_id = other_session.handle.id();

				//Send user creation packet
				self.queue_send_packet(packet_server::prepare_packet_user_create(
					other_session_id,
					&session.nick_name,
				));

				//Send current cursor positions of the session
				self.queue_send_packet(packet_server::prepare_packet_user_cursor_pos(
					other_session_id,
					session.cursor_pos.x,
					session.cursor_pos.y,
				));
			}
		}
	}

	async fn process_command_message(&mut self, _reader: &mut BinaryReader) -> Result<(), UserError> {
		Err(UserError::new("Messages not supported yet"))
	}

	async fn process_command_ping(&mut self, _reader: &mut BinaryReader) {
		// Ignore
	}

	async fn process_command_cursor_pos(
		&mut self,
		reader: &mut BinaryReader,
		_session_handle: &SessionHandle,
	) -> anyhow::Result<()> {
		self.cursor_pos_prev = self.cursor_pos.clone();
		self.cursor_pos = packet_client::PacketCursorPos::read(reader)?;
		self.update_cursor().await;

		Ok(())
	}

	async fn process_command_cursor_down(
		&mut self,
		_reader: &mut BinaryReader,
	) -> Result<(), UserError> {
		//log::trace!("Cursor down");
		if self.cursor_down {
			// Already pressed down
			return Ok(());
		}

		self.cursor_down = true;
		self.cursor_just_clicked = true;
		self.history_create_snapshot().await;
		self.update_cursor().await;

		Ok(())
	}

	async fn process_command_cursor_up(
		&mut self,
		_reader: &mut BinaryReader,
	) -> Result<(), UserError> {
		//log::trace!("Cursor up");
		if !self.cursor_down {
			return Ok(());
		}

		self.cursor_down = false;
		self.update_cursor().await;
		Ok(())
	}

	async fn process_command_boundary(&mut self, _reader: &mut BinaryReader) {
		// TODO
	}

	async fn process_command_chunks_received(&mut self, _reader: &mut BinaryReader) {
		// TODO
	}

	async fn process_command_preview_request(&mut self, _reader: &mut BinaryReader) {
		// TODO
	}

	async fn process_command_tool_size(&mut self, reader: &mut BinaryReader) -> anyhow::Result<()> {
		let size = reader.read_u8()?;
		//log::trace!("Tool size {}", size);
		if size > limits::TOOL_SIZE_MAX {
			Err(UserError::new("Invalid tool size"))?;
		}

		self.tool.size = size;
		Ok(())
	}

	async fn process_command_tool_color(&mut self, reader: &mut BinaryReader) -> anyhow::Result<()> {
		let red = reader.read_u8()?;
		let green = reader.read_u8()?;
		let blue = reader.read_u8()?;
		//log::trace!("Tool color {} {} {}", red, green, blue);

		self.tool.color = packet_client::Color {
			r: red,
			g: green,
			b: blue,
		};

		Ok(())
	}

	async fn process_command_tool_type(&mut self, reader: &mut BinaryReader) -> anyhow::Result<()> {
		let tool_type_num = reader.read_u8()?;

		if let Ok(tool_type) = packet_client::ToolType::try_from(tool_type_num) {
			self.tool.tool_type = Some(tool_type);
		} else {
			Err(UserError::new("Invalid tool type"))?;
		}

		Ok(())
	}

	async fn process_command_undo(&mut self, _reader: &mut BinaryReader) {
		// TODO
	}

	async fn fetch_room(&self) -> Option<MutexGuard<RoomInstance>> {
		if let Some(room_mtx) = &self.room_mtx {
			let room = room_mtx.lock().await;
			return Some(room);
		}
		None
	}

	async fn send_cursor_pos_to_all(&mut self, session_handle: &SessionHandle) {
		if let Some(room) = self.fetch_room().await {
			// Send our cursor position to other sessions
			room
				.broadcast(
					&packet_server::prepare_packet_user_cursor_pos(
						session_handle.id(),
						self.cursor_pos.x,
						self.cursor_pos.y,
					),
					Some(session_handle),
				)
				.await;
		}
		self.cursor_pos_sent = Some(self.cursor_pos.clone());
	}

	pub async fn tick_cursor(&mut self, session_handle: &SessionHandle) {
		if let Some(cursor_pos_sent) = &self.cursor_pos_sent {
			if self.cursor_pos != *cursor_pos_sent {
				self.send_cursor_pos_to_all(session_handle).await;
			}
		} else {
			self.send_cursor_pos_to_all(session_handle).await;
		}
	}

	fn is_chunk_linked(&self, chunk_pos: IVec2) -> bool {
		for chunk in &self.linked_chunks {
			if chunk.pos == chunk_pos {
				return true;
			}
		}
		false
	}

	fn link_chunk(&mut self, linked_chunk: LinkedChunk) {
		self.linked_chunks.push(linked_chunk);
	}

	pub async fn tick_boundary_check(&mut self) {
		if !self.needs_boundary_test {
			return;
		}

		if self.boundary.zoom > limits::BOUNDARY_ZOOM_MIN {
			let mut chunks_to_load: Vec<IVec2> = Vec::new();

			// Check which chunks aren't announced for this session
			for y in self.boundary.start_y..self.boundary.end_y {
				for x in self.boundary.start_x..self.boundary.end_x {
					if !self.is_chunk_linked(IVec2 { x, y }) {
						chunks_to_load.push(IVec2 { x, y });
					}
				}
			}

			if !chunks_to_load.is_empty() {
				let in_queue: u32 = (self.chunks_sent as i32 - self.chunks_received as i32) as u32;
				let to_send: u32 = 20 - in_queue; // Max 20 queued chunks

				for _iterations in 0..to_send {
					// Get closest chunk (circular loading)
					let center_x = self.cursor_pos.x as f32 / limits::CHUNK_SIZE_PX as f32;
					let center_y = self.cursor_pos.y as f32 / limits::CHUNK_SIZE_PX as f32;

					let mut closest_position = IVec2 { x: 0, y: 0 };
					let mut closest_distance: f32 = f32::MAX;

					for ch in &chunks_to_load {
						let distance = util::distance(center_x, center_y, ch.x as f32, ch.y as f32);
						if distance < closest_distance {
							closest_distance = distance;
							closest_position = *ch;
						}
					}

					for (idx, p) in chunks_to_load.iter().enumerate() {
						if *p == closest_position {
							chunks_to_load.remove(idx);
							break;
						}
					}

					// Announce chunk
					self.chunks_sent += 1;

					if let Some(chunk_system_mtx) = &self.chunk_system_mtx {
						let chunk_system = chunk_system_mtx.lock().await;
						let chunk_mtx = chunk_system.get_chunk_mtx(closest_position).clone();
						let mut chunk = chunk_mtx.lock().await;
						chunk.link_session(self);
						let linked_chunk = LinkedChunk::new(&closest_position, &Arc::downgrade(&chunk_mtx));
						drop(chunk_system);
						self.link_chunk(linked_chunk);
					}
				}
			}
		}
	}

	pub async fn tick_chunks_cleanup(&mut self) {}
}

impl Drop for SessionInstance {
	fn drop(&mut self) {
		assert!(self.cleaned_up, "cleanup() not called");
		log::trace!("Session dropped");
	}
}

pub type SessionInstanceMutex = Arc<Mutex<SessionInstance>>;
pub type SessionInstanceWeak = Weak<Mutex<SessionInstance>>;
gen_id!(SessionVec, SessionInstanceMutex, SessionCell, SessionHandle);
