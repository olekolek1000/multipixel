use std::error::Error;
use std::fmt;

use binary_reader::BinaryReader;
use futures_util::SinkExt;
use tokio_websockets::Message;

use crate::packet_client::ClientCmd;
use crate::{gen_id, packet_client, packet_server, Connection};

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

pub struct SessionInstance {
	pub nickname: String, // Max 255 characters

	pub cursor_pos: packet_client::PacketCursorPos,
	pub is_down: bool,

	pub kicked: bool,
}

impl SessionInstance {
	pub fn new() -> Self {
		Self {
			nickname: String::new(),
			cursor_pos: Default::default(),
			is_down: false,
			kicked: false,
		}
	}

	async fn parse_command(
		&mut self,
		command: ClientCmd,
		reader: &mut BinaryReader,
		connection: &mut Connection,
		session_id: u32,
	) -> anyhow::Result<()> {
		match command {
			ClientCmd::Announce => {
				self
					.process_command_announce(reader, connection, session_id)
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
			ClientCmd::ToolSize => self.process_command_tool_size(reader).await,
			ClientCmd::ToolColor => self.process_command_tool_color(reader).await,
			ClientCmd::ToolType => self.process_command_tool_type(reader).await,
			ClientCmd::Undo => self.process_command_undo(reader).await,
		}

		Ok(())
	}

	pub async fn process_payload(
		&mut self,
		session_id: u32,
		data: &[u8],
		connection: &mut Connection,
	) -> anyhow::Result<()> {
		if let Err(e) = self
			.process_payload_wrap(session_id, data, connection)
			.await
		{
			if e.is::<UserError>() {
				log::error!("User error: {}", e);
				let _ = self
					.kick(connection, format!("User error: {}", e).as_str())
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
		session_id: u32,
		data: &[u8],
		connection: &mut Connection,
	) -> anyhow::Result<()> {
		let mut reader = BinaryReader::from_u8(data);
		reader.set_endian(binary_reader::Endian::Big);

		let command = ClientCmd::try_from(reader.read_u16()?)?;

		//log::trace!("Session ID: {}, command {:?}", session_id, command);
		self
			.parse_command(command, &mut reader, connection, session_id)
			.await?;

		Ok(())
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
		&self,
		connection: &mut Connection,
		cause: &str,
	) -> Result<(), tokio_websockets::Error> {
		self
			.send_packet(connection, packet_server::prepare_packet_kick(cause))
			.await?;
		Ok(())
	}

	async fn process_command_announce(
		&mut self,
		reader: &mut BinaryReader,
		connection: &mut Connection,
		session_id: u32,
	) -> anyhow::Result<()> {
		let packet = packet_client::PacketAnnounce::read(reader)?;
		log::info!(
			"Session announced as {}, requested room name {}",
			packet.nickname,
			packet.room_name
		);

		self
			.send_packet(
				connection,
				packet_server::prepare_packet_your_id(session_id),
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
		self.cursor_pos = packet_client::PacketCursorPos::read(reader)?;
		//log::trace!("Cursor pos {}x{}", self.cursor_pos.x, self.cursor_pos.y);
		Ok(())
	}

	async fn process_command_cursor_down(
		&mut self,
		_reader: &mut BinaryReader,
	) -> Result<(), UserError> {
		//log::trace!("Cursor down");
		if !self.is_down {
			self.is_down = true;
		}
		Ok(())
	}

	async fn process_command_cursor_up(
		&mut self,
		_reader: &mut BinaryReader,
	) -> Result<(), UserError> {
		//log::trace!("Cursor up");
		if self.is_down {
			self.is_down = false;
		}
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

	async fn process_command_tool_size(&mut self, _reader: &mut BinaryReader) {
		// TODO
	}

	async fn process_command_tool_color(&mut self, _reader: &mut BinaryReader) {
		// TODO
	}

	async fn process_command_tool_type(&mut self, _reader: &mut BinaryReader) {
		// TODO
	}

	async fn process_command_undo(&mut self, _reader: &mut BinaryReader) {
		// TODO
	}
}

gen_id!(SessionVec, SessionInstance, SessionCell, SessionHandle);
