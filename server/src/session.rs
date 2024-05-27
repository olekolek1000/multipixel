use binary_reader::BinaryReader;
use futures_util::SinkExt;
use std::error::Error;
use std::sync::{Arc, Weak};
use std::{fmt, io};
use tokio::sync::Mutex;
use tokio_websockets::Message;

use crate::packet_client::ClientCmd;
use crate::room::RoomInstanceMutex;
use crate::server::ServerMutex;
use crate::{gen_id, limits, packet_client, packet_server, Connection};

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

#[derive(Default)]
struct ToolData {
	size: u8,
	color: packet_client::Color,
	tool_type: Option<packet_client::ToolType>,
}

pub struct SessionInstance {
	pub nickname: String, // Max 255 characters

	cursor_pos: packet_client::PacketCursorPos,
	cursor_pos_prev: packet_client::PacketCursorPos,
	cursor_down: bool,
	cursor_just_clicked: bool,

	tool: ToolData,

	room_mtx: Option<RoomInstanceMutex>,

	kicked: bool,
	announced: bool,
	cleaned_up: bool,
}

impl SessionInstance {
	pub fn new() -> Self {
		Self {
			nickname: String::new(),
			cursor_pos: Default::default(),
			cursor_pos_prev: Default::default(),
			cursor_down: false,
			cursor_just_clicked: false,
			kicked: false,
			announced: false,
			tool: Default::default(),
			room_mtx: Default::default(),
			cleaned_up: false,
		}
	}

	async fn parse_command(
		&mut self,
		command: ClientCmd,
		reader: &mut BinaryReader,
		connection: &mut Connection,
		session_handle: &SessionHandle,
		server_mtx: &ServerMutex,
	) -> anyhow::Result<()> {
		match command {
			ClientCmd::Announce => {
				self
					.process_command_announce(reader, connection, session_handle, server_mtx)
					.await?
			}
			ClientCmd::Message => self.process_command_message(reader).await?,
			ClientCmd::Ping => self.process_command_ping(reader).await,
			ClientCmd::CursorPos => self.process_command_cursor_pos(reader).await?,
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
		connection: &mut Connection,
		server_mtx: &ServerMutex,
	) -> anyhow::Result<()> {
		if let Err(e) = self
			.process_payload_wrap(session_handle, data, connection, server_mtx)
			.await
		{
			if e.is::<UserError>() {
				log::error!("User error: {}", e);
				let _ = self
					.kick(connection, format!("User error: {}", e).as_str())
					.await;
			} else if e.is::<io::Error>() {
				log::error!("IO error: {}", e);
				let _ = self
					.kick(connection, format!("IO error: {}", e).as_str())
					.await;
			} else {
				log::error!("Unknown error: {}", e);
				let _ = self
					.kick(connection, "Internal server error. This is a bug.")
					.await;
				return Err(anyhow::anyhow!("Internal server error: {}", e));
			}
		}

		Ok(())
	}

	pub async fn process_payload_wrap(
		&mut self,
		session_handle: &SessionHandle,
		data: &[u8],
		connection: &mut Connection,
		server_mtx: &ServerMutex,
	) -> anyhow::Result<()> {
		let mut reader = BinaryReader::from_u8(data);
		reader.set_endian(binary_reader::Endian::Big);

		let command = ClientCmd::try_from(reader.read_u16()?)?;

		//log::trace!("Session ID: {}, command {:?}", session_id, command);
		self
			.parse_command(command, &mut reader, connection, session_handle, server_mtx)
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

	async fn send_packet(
		&self,
		connection: &mut Connection,
		packet: packet_server::Packet,
	) -> Result<(), tokio_websockets::Error> {
		connection.send(Message::binary(packet.data)).await?;
		Ok(())
	}

	async fn kick(
		&mut self,
		connection: &mut Connection,
		cause: &str,
	) -> Result<(), tokio_websockets::Error> {
		if self.kicked {
			//Enough
			return Ok(());
		}
		self
			.send_packet(connection, packet_server::prepare_packet_kick(cause))
			.await?;
		self.kicked = true;
		Ok(())
	}

	pub async fn cleanup(&mut self, session_handle: &SessionHandle) {
		// only this for now
		self.cleanup_leave_room(session_handle).await;

		self.cleaned_up = true;
	}

	pub async fn cleanup_leave_room(&mut self, session_handle: &SessionHandle) {
		if let Some(room_mtx) = &self.room_mtx {
			room_mtx.lock().await.remove_session(session_handle);
		}
	}

	async fn process_command_announce(
		&mut self,
		reader: &mut BinaryReader,
		connection: &mut Connection,
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

		// Load room
		{
			let mut server = server_mtx.lock().await;

			self.room_mtx = Some(
				server
					.add_session_to_room(&packet.room_name, session_handle)
					.await,
			);
		}

		self
			.send_packet(
				connection,
				packet_server::prepare_packet_your_id(session_handle.id()),
			)
			.await?;

		Ok(())
	}

	async fn process_command_message(&mut self, _reader: &mut BinaryReader) -> Result<(), UserError> {
		Err(UserError::new("Messages not supported yet"))
	}

	async fn process_command_ping(&mut self, _reader: &mut BinaryReader) {
		// Ignore
	}

	async fn process_command_cursor_pos(&mut self, reader: &mut BinaryReader) -> anyhow::Result<()> {
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
}

impl Drop for SessionInstance {
	fn drop(&mut self) {
		assert!(self.cleaned_up, "cleanup() not called");
	}
}

pub type SessionInstanceMutex = Arc<Mutex<SessionInstance>>;
pub type SessionInstanceWeak = Weak<Mutex<SessionInstance>>;
gen_id!(SessionVec, SessionInstanceMutex, SessionCell, SessionHandle);
