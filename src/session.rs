use crate::canvas_cache::CanvasCache;
use crate::chunk::{ChunkInstanceMutex, ChunkInstanceWeak, ChunkPixel};
use crate::chunk_cache::ChunkCache;
use crate::chunk_system::{ChunkSystem, ChunkSystemMutex};
use crate::event_queue::EventQueue;
use crate::limits::CHUNK_SIZE_PX;
use crate::packet_client::ClientCmd;
use crate::pixel::{Color, GlobalPixel};
use crate::preview_system::PreviewSystemMutex;
use crate::room::RoomInstanceMutex;
use crate::server::ServerMutex;
use crate::tool::brush::BrushShapes;
use crate::{gen_id, limits, packet_client, packet_server, util, ConnectionWriter};
use binary_reader::BinaryReader;
use futures_util::SinkExt;
use glam::IVec2;
use std::collections::{HashSet, VecDeque};
use std::error::Error;
use std::sync::{Arc, Weak};
use std::time::Duration;
use std::{fmt, io};
use tokio::sync::{Mutex, Notify};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tokio_websockets::Message;

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
struct FloodfillTask {
	to_replace: Color,
	start_pos: IVec2,
	stack: Vec<IVec2>,
	affected_chunks: HashSet<IVec2>,
	pixels_changed: Vec<GlobalPixel>,
	canvas_cache: CanvasCache,
}

struct ToolData {
	pub size: u8,
	pub color: Color,
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
			pos: *pos,
			chunk: chunk.clone(),
			outside_boundary_duration: 0,
		}
	}
}

struct HistoryCell {
	pixels: Vec<GlobalPixel>,
}

#[derive(Default)]
struct Boundary {
	start_x: i32,
	start_y: i32,
	end_x: i32,
	end_y: i32,
	zoom: f32,
}

pub struct RoomRefs {
	room_mtx: RoomInstanceMutex,
	chunk_system_mtx: ChunkSystemMutex,
	preview_system_mtx: PreviewSystemMutex,
	brush_shapes_mtx: Arc<Mutex<BrushShapes>>,
}

pub struct SessionInstance {
	pub nick_name: String, // Max 255 characters

	admin_mode: bool,

	notifier: Arc<Notify>,
	pub queue_send: EventQueue<packet_server::Packet>,

	cancel_token: CancellationToken,

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

	chunk_cache: ChunkCache,

	room_refs: Option<Arc<RoomRefs>>,

	history_cells: Vec<HistoryCell>,

	tool: ToolData,

	kicked: bool,
	announced: bool,
	cleaned_up: bool,

	writer: Option<Arc<Mutex<ConnectionWriter>>>,
	task_sender: Option<JoinHandle<()>>,
	task_tick: Option<JoinHandle<()>>,
}

impl SessionInstance {
	pub fn new(cancel_token: CancellationToken) -> Self {
		let notifier = Arc::new(Notify::new());

		Self {
			cancel_token,
			nick_name: String::new(),
			cursor_pos: Default::default(),
			cursor_pos_prev: Default::default(),
			cursor_pos_sent: None,
			cursor_down: false,
			cursor_just_clicked: false,
			kicked: false,
			announced: false,
			needs_boundary_test: false,
			room_refs: None,
			chunk_cache: Default::default(),
			tool: Default::default(),
			notifier: notifier.clone(),
			queue_send: EventQueue::new(notifier),
			chunks_received: 0,
			chunks_sent: 0,
			linked_chunks: Default::default(),
			boundary: Default::default(),
			history_cells: Default::default(),
			cleaned_up: false,
			task_sender: None,
			task_tick: None,
			writer: None,
			admin_mode: false,
		}
	}

	async fn tick_task_runner(
		session_weak: SessionInstanceWeak,
		session_handle: &SessionHandle,
	) -> anyhow::Result<()> {
		let mut ticks: u32 = 0;
		while let Some(session_mtx) = session_weak.upgrade() {
			let mut session = session_mtx.lock().await;
			if session.cleaned_up {
				break;
			}

			if let Some(room_refs) = &session.room_refs {
				let room_refs = room_refs.clone();
				session.tick_cursor(&room_refs, session_handle).await;
				if let Err(e) = session
					.tick_boundary_check(&room_refs, session_handle, &session_mtx)
					.await
				{
					session.handle_error(&e).await?;
				}

				if ticks % 20 == 0 {
					session.tick_chunks_cleanup(session_handle).await;
				}
			}

			ticks += 1;

			drop(session);

			// TODO (low priority): calculate execution time instead of ticking every 50ms
			tokio::time::sleep(Duration::from_millis(50)).await;
		}

		Ok(())
	}

	pub fn launch_task_tick(
		session_handle: SessionHandle,
		session: &mut SessionInstance,
		session_weak: SessionInstanceWeak,
	) {
		session.task_tick = Some(
			tokio::task::Builder::new()
				.name(format!("Session {} tick task", session_handle.id()).as_str())
				.spawn(async move {
					if let Err(e) = SessionInstance::tick_task_runner(session_weak, &session_handle).await {
						log::error!("Session tick task ended abnormally: {}", e);
					} else {
						log::trace!("Session tick task ended gracefully");
					}
				})
				.unwrap(),
		);
	}

	pub fn launch_task_sender(
		session_id: u32,
		session: &mut SessionInstance,
		session_weak: SessionInstanceWeak,
		writer: ConnectionWriter,
	) {
		session.writer = Some(Arc::new(Mutex::new(writer)));
		session.task_sender = Some(
			tokio::task::Builder::new()
				.name(format!("Session {} sender task", session_id).as_str())
				.spawn(async move {
					while let Some(session_mtx) = session_weak.upgrade() {
						let mut session = session_mtx.lock().await;

						let notifier = session.notifier.clone();

						drop(session);

						notifier.notified().await;

						session = session_mtx.lock().await;
						let _ = session.send_all().await;

						if session.cleaned_up {
							break; // End this task
						}
					}
					log::trace!("Session sender task ended");
				})
				.unwrap(),
		);
	}

	async fn parse_command(
		&mut self,
		command: ClientCmd,
		reader: &mut BinaryReader,
		session_weak: SessionInstanceWeak,
		session_handle: &SessionHandle,
		server_mtx: &ServerMutex,
	) -> anyhow::Result<()> {
		if command == ClientCmd::Announce {
			self
				.process_command_announce(reader, session_handle, server_mtx)
				.await?;
			return Ok(());
		}

		if let Some(refs) = &self.room_refs {
			let refs = refs.clone();
			match command {
				ClientCmd::Announce => {
					let _ = self.kick("you have already announced").await;
				}
				ClientCmd::Message => {
					self
						.process_command_message(reader, session_weak.clone(), &refs, server_mtx)
						.await?
				}
				ClientCmd::Ping => self.process_command_ping(reader).await,
				ClientCmd::CursorPos => {
					self
						.process_command_cursor_pos(&refs, reader, session_handle)
						.await?
				}
				ClientCmd::CursorDown => self.process_command_cursor_down(&refs, reader).await?,
				ClientCmd::CursorUp => self.process_command_cursor_up(&refs, reader).await?,
				ClientCmd::Boundary => self.process_command_boundary(reader).await?,
				ClientCmd::ChunksReceived => self.process_command_chunks_received(reader).await?,
				ClientCmd::PreviewRequest => self.process_command_preview_request(&refs, reader).await?,
				ClientCmd::ToolSize => self.process_command_tool_size(reader).await?,
				ClientCmd::ToolColor => self.process_command_tool_color(reader).await?,
				ClientCmd::ToolType => self.process_command_tool_type(reader).await?,
				ClientCmd::Undo => self.process_command_undo(&refs, reader).await,
			}
		}

		Ok(())
	}

	async fn handle_error(&mut self, e: &anyhow::Error) -> anyhow::Result<()> {
		if e.is::<UserError>() {
			log::error!("User error: {}", e);
			let _ = self.kick(format!("User error: {}", e).as_str()).await;
		} else if e.is::<io::Error>() {
			log::error!("IO error: {}", e);
			let _ = self.kick(format!("IO error: {}", e).as_str()).await;
		} else {
			log::error!("Unknown error: {}", e);
			let _ = self.kick("Internal server error. This is a bug.").await;
			// Pass error further
			return Err(anyhow::anyhow!("Internal server error: {}", e));
		}
		Ok(())
	}

	pub async fn process_payload(
		&mut self,
		session_weak: SessionInstanceWeak,
		session_handle: &SessionHandle,
		data: &[u8],
		server_mtx: &ServerMutex,
	) -> anyhow::Result<()> {
		if let Err(e) = self
			.process_payload_wrap(session_weak, session_handle, data, server_mtx)
			.await
		{
			self.handle_error(&e).await?;
		}

		Ok(())
	}

	pub async fn process_payload_wrap(
		&mut self,
		session_weak: SessionInstanceWeak,
		session_handle: &SessionHandle,
		data: &[u8],
		server_mtx: &ServerMutex,
	) -> anyhow::Result<()> {
		let mut reader = BinaryReader::from_u8(data);
		reader.set_endian(binary_reader::Endian::Big);

		let command = ClientCmd::try_from(reader.read_u16()?)?;

		//log::trace!("Session ID: {}, command {:?}", session_id, command);
		self
			.parse_command(
				command,
				&mut reader,
				session_weak,
				session_handle,
				server_mtx,
			)
			.await?;

		Ok(())
	}

	fn history_create_snapshot(&mut self) {
		if self.history_cells.len() > 20 {
			self.history_cells.remove(0);
		}

		self.history_cells.push(HistoryCell {
			pixels: Default::default(),
		});
	}

	fn history_add_pixel(&mut self, pixel: GlobalPixel) {
		if self.history_cells.is_empty() {
			self.history_create_snapshot();
		}

		if let Some(last) = self.history_cells.last_mut() {
			last.pixels.push(pixel);
		}
	}

	async fn update_cursor(&mut self, refs: &RoomRefs) {
		if let Some(tool_type) = &self.tool.tool_type {
			match tool_type {
				packet_client::ToolType::Brush => self.update_cursor_brush(refs).await,
				packet_client::ToolType::Fill => self.update_cursor_fill(refs).await,
			}
		}

		self.cursor_just_clicked = false;
	}

	async fn floodfill_check_color(
		&mut self,
		refs: &RoomRefs,
		task: &mut FloodfillTask,
		global_pos: IVec2,
	) -> bool {
		let color = task
			.canvas_cache
			.get_pixel(&refs.chunk_system_mtx, &global_pos)
			.await;

		if color == self.tool.color {
			return false;
		}

		if task.to_replace != color {
			return false;
		}

		true
	}

	async fn update_cursor_fill(&mut self, refs: &RoomRefs) {
		// Allow single click only
		if !self.cursor_just_clicked {
			return;
		}

		let global_pos = IVec2::new(self.cursor_pos.x, self.cursor_pos.y);

		if !self.is_chunk_linked(ChunkSystem::global_pixel_pos_to_chunk_pos(global_pos)) {
			return;
		}

		if let Some(color) = self.get_pixel_global(refs, global_pos).await {
			if self.tool.color == color {
				return; // Nothing to do.
			}

			let mut task = FloodfillTask {
				to_replace: color,
				start_pos: global_pos,
				..Default::default()
			};

			// Plant a seed
			task.stack.push(global_pos);

			// Process as long as there are pixels to fill left
			loop {
				if let Some(cell) = task.stack.pop() {
					if i32::abs(task.start_pos.x - cell.x) > limits::FLOODFILL_MAX_DISTANCE as i32
						|| i32::abs(task.start_pos.y - cell.y) > limits::FLOODFILL_MAX_DISTANCE as i32
					{
						continue;
					}

					let pixel = GlobalPixel {
						pos: cell,
						color: self.tool.color.clone(),
					};

					task.canvas_cache.set_pixel(&pixel.pos, &pixel.color);
					task.pixels_changed.push(pixel);

					if self
						.floodfill_check_color(refs, &mut task, IVec2::new(cell.x - 1, cell.y))
						.await
					{
						task.stack.push(IVec2::new(cell.x - 1, cell.y));
					}

					if self
						.floodfill_check_color(refs, &mut task, IVec2::new(cell.x + 1, cell.y))
						.await
					{
						task.stack.push(IVec2::new(cell.x + 1, cell.y));
					}

					if self
						.floodfill_check_color(refs, &mut task, IVec2::new(cell.x, cell.y - 1))
						.await
					{
						task.stack.push(IVec2::new(cell.x, cell.y - 1));
					}

					if self
						.floodfill_check_color(refs, &mut task, IVec2::new(cell.x, cell.y + 1))
						.await
					{
						task.stack.push(IVec2::new(cell.x, cell.y + 1));
					}

					let chunk_pos = ChunkSystem::global_pixel_pos_to_chunk_pos(IVec2::new(cell.x, cell.y));
					task.affected_chunks.insert(chunk_pos);

					continue;
				}
				break;
			}

			self
				.set_pixels_global(refs, &task.pixels_changed, true)
				.await;
		}
	}

	async fn update_cursor_brush(&mut self, refs: &RoomRefs) {
		if !self.cursor_down {
			return;
		}

		let iters = std::cmp::max(
			1,
			util::distance(
				self.cursor_pos_prev.x as f32,
				self.cursor_pos_prev.y as f32,
				self.cursor_pos.x as f32,
				self.cursor_pos.y as f32,
			) as u32,
		);

		if iters > 250 {
			// Too much pixels at one iteration, stop drawing (prevent griefing and server overload)
			self.cursor_down = false;
			return;
		}

		let mut brush_shapes = refs.brush_shapes_mtx.lock().await;
		let mut pixels: Vec<GlobalPixel> = Vec::new();
		let shape_filled = brush_shapes.get_filled(self.tool.size);
		let shape_outline = brush_shapes.get_outline(self.tool.size);
		drop(brush_shapes);

		// Draw line
		for i in 0..iters {
			let alpha = (i as f64) / iters as f64;

			//Lerp
			let x = util::lerp(
				alpha,
				self.cursor_pos_prev.x as f64,
				self.cursor_pos.x as f64,
			) as i32;

			let y = util::lerp(
				alpha,
				self.cursor_pos_prev.y as f64,
				self.cursor_pos.y as f64,
			)
			.round() as i32;

			match self.tool.size {
				1 => GlobalPixel::insert_to_vec(&mut pixels, x, y, &self.tool.color),
				2 => {
					GlobalPixel::insert_to_vec(&mut pixels, x, y, &self.tool.color);
					GlobalPixel::insert_to_vec(&mut pixels, x - 1, y, &self.tool.color);
					GlobalPixel::insert_to_vec(&mut pixels, x + 1, y, &self.tool.color);
					GlobalPixel::insert_to_vec(&mut pixels, x, y - 1, &self.tool.color);
					GlobalPixel::insert_to_vec(&mut pixels, x, y + 1, &self.tool.color);
				}
				_ => {
					let shape = if i == 0 {
						&shape_filled
					} else {
						&shape_outline
					};
					for yy in 0..shape.size {
						for xx in 0..shape.size {
							unsafe {
								if *shape
									.data
									.get_unchecked(yy as usize * shape.size as usize + xx as usize)
									== 1
								{
									let pos_x = x as i32 + xx as i32 - (self.tool.size / 2) as i32;
									let pos_y = y as i32 + yy as i32 - (self.tool.size / 2) as i32;
									GlobalPixel::insert_to_vec(&mut pixels, pos_x, pos_y, &self.tool.color);
								}
							}
						}
					}
				}
			}
		}

		self.set_pixels_global(refs, &pixels, true).await;
	}

	async fn get_pixel_global(&mut self, refs: &RoomRefs, global_pos: IVec2) -> Option<Color> {
		let chunk_pos = ChunkSystem::global_pixel_pos_to_chunk_pos(global_pos);

		if let Some(chunk) = self
			.chunk_cache
			.get(&refs.chunk_system_mtx, chunk_pos)
			.await
		{
			let mut chunk = chunk.lock().await;
			let local_pos = ChunkSystem::global_pixel_pos_to_local_pixel_pos(global_pos);
			chunk.allocate_image();
			return Some(chunk.get_pixel(local_pos));
		}

		None
	}

	async fn set_pixels_global(
		&mut self,
		refs: &RoomRefs,
		pixels: &[GlobalPixel],
		with_history: bool,
	) {
		struct ChunkCacheCell {
			chunk_pos: IVec2,
			chunk: ChunkInstanceMutex,
			queued_pixels: Vec<(
				ChunkPixel, /* local pixel position */
				IVec2,      /* global pixel position */
			)>,
		}

		let mut affected_chunks: Vec<ChunkCacheCell> = Vec::new();

		fn fetch_cell<'a>(
			affected_chunks: &'a mut [ChunkCacheCell],
			chunk_pos: &IVec2,
		) -> Option<&'a mut ChunkCacheCell> {
			affected_chunks
				.iter_mut()
				.find(|cell| cell.chunk_pos == *chunk_pos)
		}

		async fn cache_new_chunk(
			chunk_cache: &mut ChunkCache,
			refs: &RoomRefs,
			affected_chunks: &mut Vec<ChunkCacheCell>,
			chunk_pos: &IVec2,
		) {
			if let Some(chunk) = chunk_cache.get(&refs.chunk_system_mtx, *chunk_pos).await {
				affected_chunks.push(ChunkCacheCell {
					chunk_pos: *chunk_pos,
					chunk,
					queued_pixels: Vec::new(),
				});
			}
		}

		// Generate affected chunks list
		for pixel in pixels {
			let chunk_pos = ChunkSystem::global_pixel_pos_to_chunk_pos(pixel.pos);
			if fetch_cell(&mut affected_chunks, &chunk_pos).is_none() {
				cache_new_chunk(
					&mut self.chunk_cache,
					refs,
					&mut affected_chunks,
					&chunk_pos,
				)
				.await;
			}
		}

		// Send pixels to chunks
		for pixel in pixels {
			let chunk_pos = ChunkSystem::global_pixel_pos_to_chunk_pos(pixel.pos);
			if let Some(cell) = fetch_cell(&mut affected_chunks, &chunk_pos) {
				cell.queued_pixels.push((
					ChunkPixel {
						color: pixel.color.clone(),
						pos: ChunkSystem::global_pixel_pos_to_local_pixel_pos(pixel.pos),
					},
					pixel.pos,
				));
			} else {
				// Skip pixel, already set
				continue;
			}
		}

		// For every affected chunk
		for cell in &affected_chunks {
			if cell.queued_pixels.is_empty() {
				continue;
			}

			let mut chunk = cell.chunk.lock().await;
			chunk.allocate_image();

			if with_history {
				for (local_pos, global_pos) in &cell.queued_pixels {
					let color = chunk.get_pixel(local_pos.pos);

					if local_pos.color != color {
						self.history_add_pixel(GlobalPixel {
							pos: *global_pos,
							color,
						});
					}
				}
			}

			let queued_pixels: Vec<ChunkPixel> = cell.queued_pixels.iter().map(|c| c.0.clone()).collect();

			let threshold = CHUNK_SIZE_PX * (CHUNK_SIZE_PX / 5); // over 1/5th of chunk modified
			let send_whole_chunk = cell.queued_pixels.len() > threshold as usize;
			chunk.set_pixels(&queued_pixels, false, send_whole_chunk);
		}
	}

	pub async fn send_all(&mut self) -> anyhow::Result<()> {
		if let Some(writer) = &self.writer {
			let mut writer = writer.lock().await;
			let packets_to_send = self.queue_send.read_all();

			for packet in &packets_to_send {
				writer.send(Message::binary(packet.data.clone())).await?;
			}
			Ok(())
		} else {
			Err(anyhow::anyhow!("Writer is not set"))
		}
	}

	pub async fn kick(&mut self, cause: &str) -> Result<(), tokio_websockets::Error> {
		if self.kicked {
			//Enough
			return Ok(());
		}
		self
			.queue_send
			.send(packet_server::prepare_packet_kick(cause));
		self.kicked = true;
		Ok(())
	}

	pub async fn cleanup(&mut self, session_handle: &SessionHandle) {
		self.cleaned_up = true;

		// Cancel all session tasks
		if let Some(task) = &self.task_sender {
			task.abort();
		}

		if let Some(task) = &self.task_tick {
			task.abort();
		}

		if let Some(refs) = &self.room_refs {
			let refs = refs.clone();

			// Unlink all chunks
			while let Some(lchunk) = self.linked_chunks.last() {
				self
					.unlink_chunk(session_handle, lchunk.chunk.clone())
					.await;
			}

			self.leave_room(&refs, session_handle).await;
		}

		self.cancel_token.cancel();
	}

	pub async fn leave_room(&mut self, refs: &RoomRefs, session_handle: &SessionHandle) {
		let mut room = refs.room_mtx.lock().await;

		// Remove itself from the room
		room.remove_session(session_handle);

		// Announce to all other sessions that our session is leaving this room
		room.broadcast(
			&packet_server::prepare_packet_user_remove(session_handle.id()),
			Some(session_handle),
		);
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
			.add_session_to_room(room_name, self.queue_send.clone(), session_handle)
			.await?;

		let room = room_mtx.lock().await;
		let brush_shapes_mtx = room.brush_shapes.clone();
		let chunk_system_mtx = room.chunk_system.clone();
		let preview_system_mtx = room.preview_system.clone();
		drop(room);

		self.room_refs = Some(Arc::new(RoomRefs {
			room_mtx: room_mtx.clone(),
			brush_shapes_mtx,
			chunk_system_mtx,
			preview_system_mtx,
		}));

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

		self
			.queue_send
			.send(packet_server::prepare_packet_your_id(session_handle.id()));

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
		room.broadcast(
			&packet_server::prepare_packet_user_create(session_handle.id(), &self.nick_name),
			Some(session_handle),
		);

		// Send annountement packets to this session from all other existing sessions (its positions and states)
		let other_sessions = room.get_all_sessions(Some(session_handle));

		drop(room); //No more needed in this context

		for other_session in other_sessions {
			if let Some(session_mtx) = other_session.instance_mtx.upgrade() {
				let session = session_mtx.lock().await;
				let other_session_id = other_session.handle.id();

				//Send user creation packet
				self
					.queue_send
					.send(packet_server::prepare_packet_user_create(
						other_session_id,
						&session.nick_name,
					));

				//Send current cursor positions of the session
				self
					.queue_send
					.send(packet_server::prepare_packet_user_cursor_pos(
						other_session_id,
						session.cursor_pos.x,
						session.cursor_pos.y,
					));
			}
		}
	}

	async fn process_command_message(
		&mut self,
		reader: &mut BinaryReader,
		session_weak: SessionInstanceWeak,
		refs: &RoomRefs,
		server_mtx: &ServerMutex,
	) -> anyhow::Result<()> {
		let msg_len = reader.read_u16()?;
		if msg_len > 1000 {
			return Err(UserError::new(
				format!("Too long message, got {}, max allowed is {}", msg_len, 1000).as_str(),
			))?;
		}

		let msg = reader.read_bytes(msg_len as usize)?;
		let str = std::str::from_utf8(msg)?;

		if str.is_empty() {
			return Ok(()); // unexpected but ok, just ignore it
		}

		if let Some(ch) = str.chars().nth(0) {
			if ch == '/' {
				self
					.handle_chat_command(session_weak, server_mtx, refs, &str[1..])
					.await?;
			} else {
				self.handle_chat_message(refs, str).await;
			}
		}

		Ok(())
	}

	fn send_reply(&self, text: String) {
		self.queue_send.send(packet_server::prepare_packet_message(
			packet_server::MessageType::PlainText,
			text.as_str(),
		));
	}

	fn send_reply_stylized(&self, text: String) {
		self.queue_send.send(packet_server::prepare_packet_message(
			packet_server::MessageType::Stylized,
			text.as_str(),
		));
	}

	fn send_unauthenticated(&self) {
		self.send_reply_stylized(String::from("[error]Unauthenticated[/error]"));
	}

	async fn handle_chat_message(&mut self, refs: &RoomRefs, mut msg: &str) {
		msg = msg.trim();
		let msg = format!("<{}> {}", self.nick_name.as_str(), msg);
		log::info!("Chat message: {}", msg);

		// Broadcast chat message to all sessions
		let room = refs.room_mtx.lock().await;
		room.broadcast(
			&packet_server::prepare_packet_message(packet_server::MessageType::PlainText, msg.as_str()),
			None,
		);
	}

	async fn handle_chat_command(
		&mut self,
		_session_weak: SessionInstanceWeak,
		server_mtx: &ServerMutex,
		refs: &RoomRefs,
		msg: &str,
	) -> anyhow::Result<()> {
		log::info!("Command requested: {}", msg);

		let mut parts: VecDeque<&str> = msg.split(" ").collect();
		if let Some(command) = parts.pop_front() {
			let redacted = command == "admin";

			self.send_reply_stylized(format!(
				"[color=green]/{}[/color]",
				if redacted {
					String::from("&lt;redacted&gt;")
				} else {
					String::from(msg)
				}
			));

			match command {
				"help" | "?" => {
					self.send_reply_stylized(String::from(
						"
						User commands:
						[color=blue]help[/color]: [i]Show this message[/i]
						[color=blue]leave[/color]: [i]Kick yourself[/i]
						Admin commands:
						[color=red]admin[/color]: [i]Log-in as admin[/i]
						[color=red]process_preview_system[/color]: [i]Force-refresh preview system[/i]
						",
					));
				}
				"leave" => self.kick("Goodbye.").await?,
				"admin" => {
					let server = server_mtx.lock().await;
					if let Some(password) = parts.pop_front() {
						if let Some(config_password) = &server.config.admin_password {
							if config_password != password {
								self.send_reply_stylized(String::from("[color=red]Invalid password[/color]"));
							} else {
								self.admin_mode = true;
								self.send_reply_stylized(String::from("[color=blue]Authenticated[/color]"));
							}
						} else {
							self.send_reply_stylized(String::from(
								"[color=red]admin_password in settings.json is not set[/color]",
							));
						}
					} else {
						self.send_reply(String::from("Usage: admin <password>"));
					}
				}
				"process_preview_system" => {
					if !self.admin_mode {
						self.send_unauthenticated();
					} else {
						self.send_reply(String::from("Preview regeneration started"));
						let cs = Arc::downgrade(&refs.chunk_system_mtx);

						tokio::spawn(async {
							ChunkSystem::regenerate_all_previews(cs).await;
						});
					}
				}
				_ => {
					self.send_reply_stylized(String::from("[color=red]Unknown command[/color]"));
				}
			}
		}
		Ok(())
	}

	async fn process_command_ping(&mut self, _reader: &mut BinaryReader) {
		// Ignore
	}

	async fn process_command_cursor_pos(
		&mut self,
		refs: &RoomRefs,
		reader: &mut BinaryReader,
		_session_handle: &SessionHandle,
	) -> anyhow::Result<()> {
		self.cursor_pos_prev = self.cursor_pos.clone();
		self.cursor_pos = packet_client::PacketCursorPos::read(reader)?;
		self.update_cursor(refs).await;

		Ok(())
	}

	async fn process_command_cursor_down(
		&mut self,
		refs: &RoomRefs,
		_reader: &mut BinaryReader,
	) -> Result<(), UserError> {
		//log::trace!("Cursor down");
		if self.cursor_down {
			// Already pressed down
			return Ok(());
		}

		self.cursor_down = true;
		self.cursor_just_clicked = true;
		self.history_create_snapshot();
		self.update_cursor(refs).await;

		Ok(())
	}

	async fn process_command_cursor_up(
		&mut self,
		refs: &RoomRefs,
		_reader: &mut BinaryReader,
	) -> Result<(), UserError> {
		//log::trace!("Cursor up");
		if !self.cursor_down {
			return Ok(());
		}

		self.cursor_down = false;
		self.update_cursor(refs).await;
		Ok(())
	}

	async fn process_command_boundary(&mut self, reader: &mut BinaryReader) -> anyhow::Result<()> {
		let start_x = reader.read_i32()?;
		let start_y = reader.read_i32()?;
		let mut end_x = reader.read_i32()?;
		let mut end_y = reader.read_i32()?;
		let zoom = reader.read_f32()?;

		// Prevent negative boundary
		end_x = end_x.max(start_x);
		end_y = end_y.max(start_y);

		// Chunk limit, max area of 20x20 (400) chunks
		end_x = end_x.min(start_x + 20);
		end_y = end_y.min(start_y + 20);

		self.boundary.start_x = start_x;
		self.boundary.start_y = start_y;
		self.boundary.end_x = end_x;
		self.boundary.end_y = end_y;
		self.boundary.zoom = zoom;
		self.needs_boundary_test = true;

		Ok(())
	}

	async fn process_command_chunks_received(
		&mut self,
		reader: &mut BinaryReader,
	) -> anyhow::Result<()> {
		let chunks_received = reader.read_u32()?;
		if chunks_received > self.chunks_sent {
			let msg = format!(
				"\"Chunks received ({})\" value is larger than Chunks sent ({})",
				chunks_received, self.chunks_sent
			);
			return Err(UserError::new(msg.as_str()))?;
		}

		if chunks_received <= self.chunks_received {
			return Err(UserError::new("\"Chunks received\" packet not incremented"))?;
		}
		self.chunks_received = chunks_received;
		Ok(())
	}

	async fn process_command_preview_request(
		&mut self,
		refs: &RoomRefs,
		reader: &mut BinaryReader,
	) -> anyhow::Result<()> {
		let preview_x = reader.read_i32()?;
		let preview_y = reader.read_i32()?;
		let zoom = reader.read_u8()?;

		let mut image_packet = None;

		if let Ok(Some(data)) = refs
			.preview_system_mtx
			.lock()
			.await
			.request_data(&IVec2::new(preview_x, preview_y), zoom)
			.await
		{
			image_packet = Some(packet_server::prepare_packet_preview_image(
				&IVec2::new(preview_x, preview_y),
				zoom,
				&data,
			));
		}

		if let Some(image_packet) = image_packet {
			self.queue_send.send(image_packet);
		}

		Ok(())
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

		self.tool.color = Color {
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

	fn queue_send_status_text(&self, text: &str) {
		self
			.queue_send
			.send(packet_server::prepare_packet_status_text(text));
	}

	async fn process_command_undo(&mut self, refs: &RoomRefs, _reader: &mut BinaryReader) {
		if let Some(cell) = self.history_cells.pop() {
			self.queue_send_status_text(format!("Undoing {} pixels...", cell.pixels.len()).as_str());
			self.set_pixels_global(refs, &cell.pixels, false).await;
			self.queue_send_status_text("");
		}
	}

	async fn send_cursor_pos_to_all(&mut self, refs: &RoomRefs, session_handle: &SessionHandle) {
		let room = refs.room_mtx.lock().await;
		room.broadcast(
			&packet_server::prepare_packet_user_cursor_pos(
				session_handle.id(),
				self.cursor_pos.x,
				self.cursor_pos.y,
			),
			Some(session_handle),
		);
		self.cursor_pos_sent = Some(self.cursor_pos.clone());
	}

	pub async fn tick_cursor(&mut self, refs: &RoomRefs, session_handle: &SessionHandle) {
		if let Some(cursor_pos_sent) = &self.cursor_pos_sent {
			if self.cursor_pos != *cursor_pos_sent {
				self.send_cursor_pos_to_all(refs, session_handle).await;
			}
		} else {
			self.send_cursor_pos_to_all(refs, session_handle).await;
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
		// Check if this chunk is already linked
		for ch in &self.linked_chunks {
			if ch.pos == linked_chunk.pos {
				// Already linked
				log::error!("Chunk {} is already linked", linked_chunk.pos);
			}
		}

		let chunk_pos = linked_chunk.pos;
		self.linked_chunks.push(linked_chunk);
		self
			.queue_send
			.send(packet_server::prepare_packet_chunk_create(chunk_pos));
	}

	async fn unlink_chunk(&mut self, session_handle: &SessionHandle, wchunk: ChunkInstanceWeak) {
		if let Some(chunk) = wchunk.upgrade() {
			let mut chunk = chunk.lock().await;
			chunk.unlink_session(session_handle);
			let chunk_pos = chunk.position;
			drop(chunk);
			for (idx, lchunk) in self.linked_chunks.iter().enumerate() {
				if Weak::ptr_eq(&lchunk.chunk, &wchunk) {
					self.linked_chunks.remove(idx);
					break;
				}
			}

			// Send packet to the client
			self
				.queue_send
				.send(packet_server::prepare_packet_chunk_remove(chunk_pos));
		}
	}

	pub async fn tick_boundary_check(
		&mut self,
		refs: &RoomRefs,
		session_handle: &SessionHandle,
		session: &SessionInstanceMutex,
	) -> anyhow::Result<()> {
		if !self.needs_boundary_test {
			return Ok(());
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

					if let Some(chunk_mtx) = self
						.chunk_cache
						.get(&refs.chunk_system_mtx, closest_position)
						.await
					{
						let mut chunk = chunk_mtx.lock().await;
						chunk.link_session(
							session_handle,
							Arc::downgrade(session),
							self.queue_send.clone(),
						);
						let linked_chunk = LinkedChunk::new(&closest_position, &Arc::downgrade(&chunk_mtx));
						self.link_chunk(linked_chunk);
						chunk.send_chunk_data_to_session(self.queue_send.clone());
					}

					if chunks_to_load.is_empty() {
						break;
					}
				}
			}
		}

		Ok(())
	}

	pub async fn tick_chunks_cleanup(&mut self, session_handle: &SessionHandle) {
		// Remove chunks outside bounds and left for longer time
		let mut chunks_to_unload: Vec<ChunkInstanceWeak> = Vec::new();

		for i in 0..self.linked_chunks.len() {
			let linked_chunk = &mut self.linked_chunks[i];
			if let Some(chunk) = linked_chunk.chunk.upgrade() {
				let pos = chunk.lock().await.position;
				if self.boundary.zoom <= limits::MIN_ZOOM
					|| pos.y < self.boundary.start_y
					|| pos.y > self.boundary.end_y
					|| pos.x < self.boundary.start_x
					|| pos.x > self.boundary.end_x
				{
					linked_chunk.outside_boundary_duration += 1;
					if linked_chunk.outside_boundary_duration == 5
					/* 5 seconds */
					{
						chunks_to_unload.push(linked_chunk.chunk.clone());
					}
				} else {
					linked_chunk.outside_boundary_duration = 0;
				}
			}
		}

		// Deannounce chunks from list
		for chunk in &chunks_to_unload {
			self.unlink_chunk(session_handle, chunk.clone()).await;
		}
	}
}

impl Drop for SessionInstance {
	fn drop(&mut self) {
		assert!(self.cleaned_up, "cleanup() not called");
		log::debug!("Session dropped");
	}
}

pub type SessionInstanceMutex = Arc<Mutex<SessionInstance>>;
pub type SessionInstanceWeak = Weak<Mutex<SessionInstance>>;
gen_id!(SessionVec, SessionInstanceMutex, SessionCell, SessionHandle);
