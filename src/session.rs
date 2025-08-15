use crate::canvas_cache::CanvasCache;
use crate::chunk::cache::ChunkCache;
use crate::chunk::chunk::{ChunkInstanceWeak, ChunkPixelRGBA};
use crate::chunk::compositor::LayerID;
use crate::chunk::system::ChunkSystem;
use crate::chunk::writer::ChunkWriterRGBA;
use crate::event_queue::EventQueue;
use crate::limits::CHUNK_SIZE_PX;
use crate::packet_client::ClientCmd;
use crate::pixel::{ColorRGB, ColorRGBA, GlobalPixelRGBA};
use crate::room::{RoomInstanceMutex, RoomRefs};
use crate::serial_generator::SerialGenerator;
use crate::server::ServerMutex;
use crate::tool::iter::LineMoveIter;
use crate::tool::state::{ToolState, ToolStateLine};
use crate::{gen_id, limits, packet_client, packet_server, util, ConnectionWriter};
use binary_reader::BinaryReader;
use futures_util::SinkExt;
use glam::IVec2;
use std::collections::{HashSet, VecDeque};
use std::error::Error;
use std::sync::Mutex as SyncMutex;
use std::sync::{Arc, Weak};
use std::time::Duration;
use std::{fmt, io};
use tokio::sync::{Mutex, Notify};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tokio_websockets::Message;

const SERVER_STR: &str = "[SERVER]";

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
	to_replace: ColorRGBA,
	start_pos: IVec2,
	stack: Vec<IVec2>,
	affected_chunks: HashSet<IVec2>,
	pixels_changed: Vec<GlobalPixelRGBA>,
	canvas_cache: CanvasCache,
}

struct ToolData {
	pub size: u8,
	pub flow: f32,
	pub color: ColorRGB,
	pub tool_type: Option<packet_client::ToolType>,
}

impl Default for ToolData {
	fn default() -> Self {
		Self {
			size: 1,
			flow: 0.5,
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
	pixels: Vec<GlobalPixelRGBA>,
}

#[derive(Default)]
struct Boundary {
	start_x: i32,
	start_y: i32,
	end_x: i32,
	end_y: i32,
	zoom: f32,
}

#[derive(Default)]
struct History {
	cells: Vec<HistoryCell>,
	modified_pixel_coords: HashSet<IVec2>,
}

impl History {
	fn create_snapshot(&mut self) {
		if self.cells.len() > 50 {
			self.cells.remove(0);
		}

		self.modified_pixel_coords.clear();

		self.cells.push(HistoryCell {
			pixels: Default::default(),
		});
	}

	fn add_pixel(&mut self, pixel: GlobalPixelRGBA) {
		if self.cells.is_empty() {
			self.create_snapshot();
		}

		if let Some(last) = self.cells.last_mut() {
			if self.modified_pixel_coords.insert(pixel.pos) {
				last.pixels.push(pixel);
			}
		}
	}

	fn undo(&mut self) -> Option<HistoryCell> {
		self.cells.pop()
	}
}

#[derive(Default)]
pub struct Cursor {
	pos: packet_client::PacketCursorPos,
	pos_prev: packet_client::PacketCursorPos,
	pos_sent: Option<packet_client::PacketCursorPos>,
	down: bool,
	just_clicked: bool,
}

#[derive(Default)]
pub struct SessionState {
	pub nick_name: String, // Max 255 characters
	pub cursor: Cursor,
}

pub struct SessionInstance {
	pub state: Arc<SyncMutex<SessionState>>,

	admin_mode: bool,

	notifier: Arc<Notify>,
	pub queue_send: EventQueue<packet_server::Packet>,

	cancel_token: CancellationToken,

	needs_boundary_test: bool,
	boundary: Boundary,

	linked_chunks: Vec<LinkedChunk>,
	chunks_sent: u32,     // Number of chunks received by the client
	chunks_received: u32, // Number of chunks sent by the server

	chunk_cache: ChunkCache,

	room_refs: Option<Arc<RoomRefs>>,

	history: History,

	tool: ToolData,

	kicked: bool,
	announced: bool,
	cleaned_up: bool,

	tool_state: ToolState,

	writer: Option<Arc<Mutex<ConnectionWriter>>>,
	task_sender: Option<JoinHandle<()>>,
	task_tick: Option<JoinHandle<()>>,

	serial_generator: SerialGenerator,
}

impl SessionInstance {
	pub fn new(cancel_token: CancellationToken) -> Self {
		let notifier = Arc::new(Notify::new());

		Self {
			cancel_token,
			state: Arc::new(SyncMutex::new(SessionState::default())),
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
			history: Default::default(),
			cleaned_up: false,
			task_sender: None,
			task_tick: None,
			writer: None,
			admin_mode: false,
			tool_state: ToolState::None,
			serial_generator: SerialGenerator::new(),
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

				session.tick_tool_state(&room_refs).await;

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
						log::error!("Session tick task ended abnormally: {e}");
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
				.name(format!("Session {session_id} sender task").as_str())
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
				ClientCmd::CursorDown => {
					self
						.process_command_cursor_down(&refs, reader, session_handle)
						.await?
				}
				ClientCmd::CursorUp => {
					self
						.process_command_cursor_up(&refs, reader, session_handle)
						.await?
				}
				ClientCmd::Boundary => self.process_command_boundary(reader).await?,
				ClientCmd::ChunksReceived => self.process_command_chunks_received(reader).await?,
				ClientCmd::PreviewRequest => self.process_command_preview_request(&refs, reader).await?,
				ClientCmd::ToolSize => self.process_command_tool_size(reader).await?,
				ClientCmd::ToolFlow => self.process_command_tool_flow(reader).await?,
				ClientCmd::ToolColor => self.process_command_tool_color(reader).await?,
				ClientCmd::ToolType => self.process_command_tool_type(reader).await?,
				ClientCmd::Undo => self.process_command_undo(&refs, reader).await,
			}
		}

		Ok(())
	}

	async fn handle_error(&mut self, e: &anyhow::Error) -> anyhow::Result<()> {
		if e.is::<UserError>() {
			log::error!("User error: {e}");
			let _ = self.kick(format!("User error: {e}").as_str()).await;
		} else if e.is::<io::Error>() {
			log::error!("IO error: {e}");
			let _ = self.kick(format!("IO error: {e}").as_str()).await;
		} else {
			log::error!("Unknown error: {e}");
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

	fn state(&self) -> std::sync::MutexGuard<SessionState> {
		self.state.lock().unwrap()
	}

	async fn set_tool_state(&mut self, refs: &RoomRefs, new_state: ToolState) {
		match &mut self.tool_state {
			ToolState::None => {}
			ToolState::Line(state) => {
				state.cleanup(refs).await;
				let pixels = state.gen_global_pixel_vec_rgba(self.tool.color.rgba(255));
				self.set_pixels_main(refs, &pixels, true).await;
			}
		}
		self.tool_state = new_state;
	}

	async fn update_cursor(&mut self, refs: &RoomRefs, session_handle: &SessionHandle) {
		if let Some(tool_type) = &self.tool.tool_type {
			match tool_type {
				packet_client::ToolType::Brush => self.update_cursor_brush(refs, false).await,
				packet_client::ToolType::SquareBrush => self.update_cursor_brush(refs, true).await,
				packet_client::ToolType::Line => self.update_cursor_line(refs, session_handle).await,
				packet_client::ToolType::SmoothBrush => self.update_cursor_smooth_brush(refs).await,
				packet_client::ToolType::Spray => self.update_cursor_spray(refs).await,
				packet_client::ToolType::Fill => self.update_cursor_fill(refs).await,
				packet_client::ToolType::Blur => self.update_cursor_blur(refs).await,
				packet_client::ToolType::Smudge => self.update_cursor_smudge(refs).await,
			}
		}

		self.state().cursor.just_clicked = false;
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

		if color == self.tool.color.rgba(255) {
			return false;
		}

		if task.to_replace != color {
			return false;
		}

		true
	}

	async fn update_cursor_fill(&mut self, refs: &RoomRefs) {
		let global_pos = {
			let state = self.state();

			if !state.cursor.down {
				return;
			}

			// Allow single click only
			if !state.cursor.just_clicked {
				return;
			}

			IVec2::new(state.cursor.pos.x, state.cursor.pos.y)
		};

		if !self.is_chunk_linked(ChunkSystem::global_pixel_pos_to_chunk_pos(global_pos)) {
			return;
		}

		if let Some(color) = self.get_pixel_main(refs, global_pos).await {
			if self.tool.color.rgba(255) == color {
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

					let pixel = GlobalPixelRGBA {
						pos: cell,
						color: self.tool.color.rgba(255),
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

			self.set_pixels_main(refs, &task.pixels_changed, true).await;
		}
	}

	async fn update_cursor_brush(&mut self, refs: &RoomRefs, square: bool) {
		let (tool_size, iter, step) = {
			let mut state = self.state();

			if !state.cursor.down {
				return;
			}

			let tool_size = self.get_tool_size();
			let step = 1 + tool_size / 6;
			let iter = LineMoveIter::iterate(state.cursor.pos_prev.to_vec(), state.cursor.pos.to_vec());
			if util::distance_squared_int32(state.cursor.pos_prev.to_vec(), state.cursor.pos.to_vec())
				> 250
			{
				// Too much pixels at one iteration, stop drawing (prevent griefing and server overload)
				state.cursor.down = false;
				return;
			}

			(tool_size, iter, step)
		};

		let mut brush_shapes = refs.brush_shapes_mtx.lock().await;
		let mut pixels: Vec<GlobalPixelRGBA> = Vec::new();
		let shape_filled = if square {
			brush_shapes.get_square_filled(tool_size)
		} else {
			brush_shapes.get_circle_filled(tool_size)
		};

		let shape_outline = if square {
			brush_shapes.get_square_outline(tool_size)
		} else {
			brush_shapes.get_circle_outline(tool_size)
		};
		drop(brush_shapes);

		for (index, line) in iter.enumerate() {
			if index as u32 % step as u32 != 0 {
				continue;
			}

			let tool_color = self.tool.color.rgba(255);

			match tool_size {
				1 => GlobalPixelRGBA::insert_to_vec(&mut pixels, line.pos.x, line.pos.y, &tool_color),
				2 => {
					GlobalPixelRGBA::insert_to_vec(&mut pixels, line.pos.x, line.pos.y, &tool_color);
					GlobalPixelRGBA::insert_to_vec(&mut pixels, line.pos.x - 1, line.pos.y, &tool_color);
					GlobalPixelRGBA::insert_to_vec(&mut pixels, line.pos.x + 1, line.pos.y, &tool_color);
					GlobalPixelRGBA::insert_to_vec(&mut pixels, line.pos.x, line.pos.y - 1, &tool_color);
					GlobalPixelRGBA::insert_to_vec(&mut pixels, line.pos.x, line.pos.y + 1, &tool_color);
				}
				_ => {
					let shape = if index == 0 {
						&shape_filled
					} else {
						&shape_outline
					};

					for s in shape.iterate() {
						let pos_x = line.pos.x + s.local_x as i32 - (tool_size / 2) as i32;
						let pos_y = line.pos.y + s.local_y as i32 - (tool_size / 2) as i32;
						GlobalPixelRGBA::insert_to_vec(&mut pixels, pos_x, pos_y, &tool_color);
					}
				}
			}
		}

		self.set_pixels_main(refs, &pixels, true).await;
	}

	async fn update_cursor_line(&mut self, refs: &RoomRefs, session_handle: &SessionHandle) {
		let (cursor_down, cursor_pos) = {
			let state = self.state();
			(state.cursor.down, state.cursor.pos.clone())
		};

		if cursor_down && !matches!(self.tool_state, ToolState::Line(_)) {
			// Start drawing line
			let layer_generation = self.serial_generator.increment_get();

			self
				.set_tool_state(
					refs,
					ToolState::Line(ToolStateLine::new(
						cursor_pos.to_vec(),
						LayerID::Session(layer_generation, *session_handle),
					)),
				)
				.await;
		}

		let mut ending_drawing = false;

		if !cursor_down && matches!(self.tool_state, ToolState::Line(_)) {
			// Stop drawing line
			ending_drawing = true;
		}

		if ending_drawing {
			// render for the last time
			if let ToolState::Line(state) = &mut self.tool_state {
				state
					.process(
						&mut self.chunk_cache,
						refs,
						cursor_pos.to_vec(),
						self.tool.color,
					)
					.await;
			}
			self.set_tool_state(refs, ToolState::None).await;
		}
	}

	async fn update_cursor_smooth_brush(&mut self, refs: &RoomRefs) {
		let (tool_size, iter, step) = {
			let mut state = self.state();

			if !state.cursor.down {
				return;
			}

			let tool_size = self.get_tool_size().max(4);
			let step = 1 + tool_size / 6;
			let iter = LineMoveIter::iterate(state.cursor.pos_prev.to_vec(), state.cursor.pos.to_vec());
			if util::distance_squared_int32(state.cursor.pos_prev.to_vec(), state.cursor.pos.to_vec())
				> 250
			{
				// Too much pixels at one iteration, stop drawing (prevent griefing and server overload)
				state.cursor.down = false;
				return;
			}
			(tool_size, iter, step)
		};

		let mut pixels: Vec<GlobalPixelRGBA> = Vec::new();
		let mut cache = CanvasCache::default();

		let intensity = (self.tool.flow.powf(2.0) * 255.0) as u8;
		let center_pos = (tool_size / 2) as f32 + 0.01 /* prevent NaN */;

		let mut index = 0;
		for line in iter {
			if index % step == 0 {
				for brush_y in 0..tool_size {
					for brush_x in 0..tool_size {
						let pos_x = line.pos.x + brush_x as i32 - tool_size as i32 / 2;
						let pos_y = line.pos.y + brush_y as i32 - tool_size as i32 / 2;

						let current = cache
							.get_pixel(&refs.chunk_system_mtx, &IVec2::new(pos_x, pos_y))
							.await;

						let mult = (util::distance32(brush_x as f32, brush_y as f32, center_pos, center_pos)
							/ (tool_size / 2) as f32)
							.clamp(0.0, 1.0); // normalized from 0.0 to 1.0

						if mult <= 0.0 {
							continue;
						}

						let blended = ColorRGBA::blend_gamma_corrected(
							(intensity as f32 * (1.0 - mult)) as u8,
							&current,
							&self.tool.color.rgba(255),
						);

						cache.set_pixel(&IVec2::new(pos_x, pos_y), &blended);
						GlobalPixelRGBA::insert_to_vec(&mut pixels, pos_x, pos_y, &blended);
					}
				}
			}
			index += 1;
		}

		self.set_pixels_main(refs, &pixels, true).await;
	}

	async fn update_cursor_spray(&mut self, refs: &RoomRefs) {
		let iter = {
			let mut state = self.state();

			if !state.cursor.down {
				return;
			}

			let iter = LineMoveIter::iterate(state.cursor.pos_prev.to_vec(), state.cursor.pos.to_vec());

			if util::distance_squared_int32(state.cursor.pos_prev.to_vec(), state.cursor.pos.to_vec())
				> 250
			{
				// Too much pixels at one iteration, stop drawing (prevent griefing and server overload)
				state.cursor.down = false;
				return;
			}
			iter
		};

		let tool_size = self.get_tool_size();

		let mut brush_shapes = refs.brush_shapes_mtx.lock().await;
		let mut pixels: Vec<GlobalPixelRGBA> = Vec::new();
		let shape_filled = brush_shapes.get_circle_filled(tool_size);
		drop(brush_shapes);

		let threshold = 0.001 + self.tool.flow.powf(4.0) * 0.05;

		for line in iter {
			for s in shape_filled.iterate() {
				let rand = fastrand::f32();
				if rand > threshold {
					continue;
				}

				let pos_x = line.pos.x + s.local_x as i32 - (tool_size / 2) as i32;
				let pos_y = line.pos.y + s.local_y as i32 - (tool_size / 2) as i32;
				GlobalPixelRGBA::insert_to_vec(&mut pixels, pos_x, pos_y, &self.tool.color.rgba(255));
			}
		}

		self.set_pixels_main(refs, &pixels, true).await;
	}

	async fn update_cursor_blur(&mut self, refs: &RoomRefs) {
		let (cursor_down, cursor_pos) = {
			let state = self.state();
			(state.cursor.down, state.cursor.pos.clone())
		};

		if !cursor_down {
			return;
		}

		let tool_size = self.get_tool_size();

		let shape_filled = {
			let mut brush_shapes = refs.brush_shapes_mtx.lock().await;
			brush_shapes.get_circle_filled(tool_size)
		};

		let mut pixels: Vec<GlobalPixelRGBA> = Vec::new();

		let blend_intensity = (self.tool.flow * 255.0) as u8;

		let mut cache = CanvasCache::default();

		for s in shape_filled.iterate() {
			let pos_x = cursor_pos.x + s.local_x as i32 - (tool_size / 2) as i32;
			let pos_y = cursor_pos.y + s.local_y as i32 - (tool_size / 2) as i32;

			let center = cache
				.get_pixel(&refs.chunk_system_mtx, &IVec2::new(pos_x, pos_y))
				.await;
			let left = cache
				.get_pixel(&refs.chunk_system_mtx, &IVec2::new(pos_x - 1, pos_y))
				.await;
			let right = cache
				.get_pixel(&refs.chunk_system_mtx, &IVec2::new(pos_x + 1, pos_y))
				.await;
			let top = cache
				.get_pixel(&refs.chunk_system_mtx, &IVec2::new(pos_x, pos_y - 1))
				.await;
			let bottom = cache
				.get_pixel(&refs.chunk_system_mtx, &IVec2::new(pos_x, pos_y + 1))
				.await;

			let blended_horiz = ColorRGBA::blend_gamma_corrected(127, &left, &right);
			let blended_vert = ColorRGBA::blend_gamma_corrected(127, &top, &bottom);
			let blended = ColorRGBA::blend_gamma_corrected(127, &blended_horiz, &blended_vert);
			let current = ColorRGBA::blend_gamma_corrected(blend_intensity, &center, &blended);
			GlobalPixelRGBA::insert_to_vec(&mut pixels, pos_x, pos_y, &current);
		}

		self.set_pixels_main(refs, &pixels, true).await;
	}

	async fn update_cursor_smudge(&mut self, refs: &RoomRefs) {
		let (cursor_pos, cursor_pos_prev) = {
			let state = self.state();

			if !state.cursor.down {
				return;
			}

			(state.cursor.pos.clone(), state.cursor.pos_prev.clone())
		};

		let tool_size = self.get_tool_size();

		let shape_filled = {
			let mut brush_shapes = refs.brush_shapes_mtx.lock().await;
			brush_shapes.get_circle_filled(tool_size)
		};

		let mut pixels_final: Vec<GlobalPixelRGBA> = Vec::new();
		let mut pixels_temp: Vec<GlobalPixelRGBA> = Vec::new();

		let blend_intensity = (self.tool.flow * 255.0) as u8;

		let mut cache = CanvasCache::default();

		let iter = LineMoveIter::iterate(cursor_pos_prev.to_vec(), cursor_pos.to_vec());

		let mut line_x_prev = cursor_pos_prev.x;
		let mut line_y_prev = cursor_pos_prev.y;

		for line in iter {
			let diff_x = line_x_prev - line.pos.x;
			let diff_y = line_y_prev - line.pos.y;
			if diff_x == 0 && diff_y == 0 {
				continue; // Nothing to smudge
			}

			pixels_temp.clear();
			for s in shape_filled.iterate() {
				let pos_x = line.pos.x + s.local_x as i32 - (tool_size / 2) as i32;
				let pos_y = line.pos.y + s.local_y as i32 - (tool_size / 2) as i32;

				let prev = cache
					.get_pixel(
						&refs.chunk_system_mtx,
						&IVec2::new(pos_x + diff_x, pos_y + diff_y),
					)
					.await;

				let center = cache
					.get_pixel(&refs.chunk_system_mtx, &IVec2::new(pos_x, pos_y))
					.await;

				let blended = ColorRGBA::blend_gamma_corrected(blend_intensity, &center, &prev);
				GlobalPixelRGBA::insert_to_vec(&mut pixels_temp, pos_x, pos_y, &blended);
			}

			for pixel in &pixels_temp {
				cache.set_pixel(&pixel.pos, &pixel.color);
			}
			pixels_final.append(&mut pixels_temp);

			line_x_prev = line.pos.x;
			line_y_prev = line.pos.y;
		}

		self.set_pixels_main(refs, &pixels_final, true).await;
	}

	async fn get_pixel_main(&mut self, refs: &RoomRefs, global_pos: IVec2) -> Option<ColorRGBA> {
		let chunk_pos = ChunkSystem::global_pixel_pos_to_chunk_pos(global_pos);

		if let Some(chunk) = self
			.chunk_cache
			.get(&refs.chunk_system_mtx, chunk_pos)
			.await
		{
			let mut chunk = chunk.lock().await;
			let local_pos = ChunkSystem::global_pixel_pos_to_local_pixel_pos(global_pos);
			chunk.allocate_image();
			return Some(chunk.get_pixel_main(local_pos));
		}

		None
	}

	async fn set_pixels_main(
		&mut self,
		refs: &RoomRefs,
		pixels: &[GlobalPixelRGBA],
		with_history: bool,
	) {
		let mut writer = ChunkWriterRGBA::new();

		writer
			.generate_affected(pixels, &mut self.chunk_cache, &refs.chunk_system_mtx)
			.await;

		// For every affected chunk
		for cell in &writer.affected_chunks {
			if cell.queued_pixels.is_empty() {
				continue;
			}

			let mut chunk = cell.chunk.lock().await;
			chunk.allocate_image();

			if with_history {
				for (local_pos, global_pos) in &cell.queued_pixels {
					let color = chunk.get_pixel_main(local_pos.pos);

					if local_pos.color != color {
						self.history.add_pixel(GlobalPixelRGBA {
							pos: *global_pos,
							color,
						});
					}
				}
			}

			let queued_pixels: Vec<ChunkPixelRGBA> =
				cell.queued_pixels.iter().map(|c| c.0.clone()).collect();

			let threshold = CHUNK_SIZE_PX * (CHUNK_SIZE_PX / 5); // over 1/5th of chunk modified
			let send_whole_chunk = cell.queued_pixels.len() > threshold as usize;
			chunk.set_pixels(&queued_pixels, send_whole_chunk);
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
			.add_session_to_room(
				room_name,
				self.queue_send.clone(),
				session_handle,
				self.state.clone(),
			)
			.await?;

		let room = room_mtx.lock().await;
		let brush_shapes_mtx = room.brush_shapes.clone();
		let chunk_system_mtx = room.chunk_system.clone();
		let preview_system_mtx = room.preview_system.clone();
		let chunk_system_sender = room.chunk_system_sender.clone();
		drop(room);

		self.room_refs = Some(Arc::new(RoomRefs {
			room_mtx: room_mtx.clone(),
			brush_shapes_mtx,
			chunk_system_mtx,
			preview_system_mtx,
			chunk_system_sender,
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
						"Room name contains invalid character: \"{ch}\". Only alphanumeric characters are allowed."
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
						"Nick name contains invalid character: \"{ch}\". Only alphanumeric characters are allowed."
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

		let suitable_nick = room_mtx
			.lock()
			.await
			.get_suitable_nick_name(packet.nick_name.as_str(), session_handle)
			.await;

		self.state().nick_name = suitable_nick;

		// Broadcast to all users that this user is available
		self.broadcast_self(room_mtx, session_handle).await;

		// Reset tool state
		self.tool = ToolData::default();

		Ok(())
	}

	async fn broadcast_self(&mut self, room_mtx: RoomInstanceMutex, session_handle: &SessionHandle) {
		let room = room_mtx.lock().await;

		let nick_name = self.state().nick_name.clone();

		// Announce itself to other existing sessions
		room.broadcast(
			&packet_server::prepare_packet_user_create(session_handle.id(), &nick_name),
			Some(session_handle),
		);

		// Send annountement packets to this session from all other existing sessions (its positions and states)
		let other_sessions = room.get_all_sessions(Some(session_handle));

		drop(room); //No more needed in this context

		for other_session in other_sessions {
			let other_session_id = other_session.handle.id();

			let other_state = other_session.state.lock().unwrap();

			//Send user creation packet
			self
				.queue_send
				.send(packet_server::prepare_packet_user_create(
					other_session_id,
					&other_state.nick_name,
				));

			//Send current cursor positions of the session
			self
				.queue_send
				.send(packet_server::prepare_packet_user_cursor_pos(
					other_session_id,
					other_state.cursor.pos.x,
					other_state.cursor.pos.y,
				));
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
			SERVER_STR,
			text.as_str(),
		));
	}

	fn send_reply_stylized(&self, text: String) {
		self.queue_send.send(packet_server::prepare_packet_message(
			packet_server::MessageType::Stylized,
			SERVER_STR,
			text.as_str(),
		));
	}

	fn send_unauthenticated(&self) {
		self.send_reply_stylized(String::from("[error]Unauthenticated[/error]"));
	}

	async fn handle_chat_message(&mut self, refs: &RoomRefs, mut msg: &str) {
		msg = msg.trim();
		let nick_name = self.state().nick_name.clone();
		log::info!("Chat message: {msg}");

		// Broadcast chat message to all sessions
		let room = refs.room_mtx.lock().await;
		room.broadcast(
			&packet_server::prepare_packet_message(
				packet_server::MessageType::PlainText,
				&nick_name,
				msg,
			),
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
		log::info!("Command requested: {msg}");

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
		session_handle: &SessionHandle,
	) -> anyhow::Result<()> {
		{
			let mut state = self.state();
			state.cursor.pos_prev = state.cursor.pos.clone();
			state.cursor.pos = packet_client::PacketCursorPos::read(reader)?;
		}

		self.update_cursor(refs, session_handle).await;

		let (pos_sent, cursor_pos) = {
			let state = self.state();
			(state.cursor.pos_sent.clone(), state.cursor.pos.clone())
		};

		if let Some(cursor_pos_sent) = pos_sent {
			if cursor_pos != cursor_pos_sent {
				self.send_cursor_pos_to_all(refs, session_handle).await;
			}
		} else {
			self.send_cursor_pos_to_all(refs, session_handle).await;
		}

		Ok(())
	}

	async fn process_command_cursor_down(
		&mut self,
		refs: &RoomRefs,
		_reader: &mut BinaryReader,
		session_handle: &SessionHandle,
	) -> Result<(), UserError> {
		{
			let mut state = self.state();

			//log::trace!("Cursor down");
			if state.cursor.down {
				// Already pressed down
				return Ok(());
			}

			state.cursor.pos_prev = state.cursor.pos.clone();
			state.cursor.down = true;
			state.cursor.just_clicked = true;
		}

		self.history.create_snapshot();
		self.update_cursor(refs, session_handle).await;

		Ok(())
	}

	async fn process_command_cursor_up(
		&mut self,
		refs: &RoomRefs,
		_reader: &mut BinaryReader,
		session_handle: &SessionHandle,
	) -> Result<(), UserError> {
		{
			let mut state = self.state();

			//log::trace!("Cursor up");
			if !state.cursor.down {
				return Ok(());
			}

			state.cursor.down = false;
		}

		self.update_cursor(refs, session_handle).await;
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
		self.tool.size = size;
		Ok(())
	}

	fn get_tool_size(&self) -> u8 {
		let Some(tool_type) = &self.tool.tool_type else {
			return 0;
		};

		match tool_type {
			packet_client::ToolType::Fill => 0,
			packet_client::ToolType::Line => self.tool.size.min(limits::TOOL_SIZE_LINE_MAX),
			packet_client::ToolType::Brush => self.tool.size.min(limits::TOOL_SIZE_BRUSH_MAX),
			packet_client::ToolType::SmoothBrush => {
				self.tool.size.min(limits::TOOL_SIZE_SMOOTH_BRUSH_MAX)
			}
			packet_client::ToolType::SquareBrush => {
				self.tool.size.min(limits::TOOL_SIZE_SQUARE_BRUSH_MAX)
			}
			packet_client::ToolType::Spray => self.tool.size.min(limits::TOOL_SIZE_SPRAY_MAX),
			packet_client::ToolType::Blur => self.tool.size.min(limits::TOOL_SIZE_BLUR_MAX),
			packet_client::ToolType::Smudge => self.tool.size.min(limits::TOOL_SIZE_SMUDGE_MAX),
		}
	}

	async fn process_command_tool_flow(&mut self, reader: &mut BinaryReader) -> anyhow::Result<()> {
		let flow = reader.read_f32()?;
		if !flow.is_finite() {
			Err(UserError::new("Invalid tool flow"))?;
		}

		self.tool.flow = flow.clamp(0.0, 1.0);
		Ok(())
	}

	async fn process_command_tool_color(&mut self, reader: &mut BinaryReader) -> anyhow::Result<()> {
		let red = reader.read_u8()?;
		let green = reader.read_u8()?;
		let blue = reader.read_u8()?;
		//log::trace!("Tool color {} {} {}", red, green, blue);

		self.tool.color = ColorRGB {
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
		if let Some(cell) = self.history.undo() {
			self.queue_send_status_text(format!("Undoing {} pixels...", cell.pixels.len()).as_str());
			self.set_pixels_main(refs, &cell.pixels, false).await;
			self.queue_send_status_text("");
		}
	}

	async fn send_cursor_pos_to_all(&mut self, refs: &RoomRefs, session_handle: &SessionHandle) {
		let cursor_pos = {
			let mut state = self.state();
			state.cursor.pos_sent = Some(state.cursor.pos.clone());
			state.cursor.pos.clone()
		};

		let room = refs.room_mtx.lock().await;
		room.broadcast(
			&packet_server::prepare_packet_user_cursor_pos(
				session_handle.id(),
				cursor_pos.x,
				cursor_pos.y,
			),
			Some(session_handle),
		);
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

	pub async fn tick_tool_state(&mut self, refs: &RoomRefs) {
		let cursor_pos = self.state().cursor.pos.clone();

		match &mut self.tool_state {
			ToolState::None => {}
			ToolState::Line(state) => {
				state
					.process(
						&mut self.chunk_cache,
						refs,
						cursor_pos.to_vec(),
						self.tool.color,
					)
					.await;
			}
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
		if self.boundary.zoom <= limits::BOUNDARY_ZOOM_MIN {
			return Ok(());
		}

		let mut chunks_to_load: Vec<IVec2> = Vec::new();

		// Check which chunks aren't announced for this session
		for y in self.boundary.start_y..self.boundary.end_y {
			for x in self.boundary.start_x..self.boundary.end_x {
				if !self.is_chunk_linked(IVec2 { x, y }) {
					chunks_to_load.push(IVec2 { x, y });
				}
			}
		}

		if chunks_to_load.is_empty() {
			return Ok(());
		}

		let cursor_pos = self.state().cursor.pos.clone();

		let in_queue: u32 = (self.chunks_sent as i32 - self.chunks_received as i32) as u32;
		let to_send: u32 = 20 - in_queue; // Max 20 queued chunks

		for _iterations in 0..to_send {
			// Get closest chunk (circular loading)
			let center_x = cursor_pos.x as f64 / limits::CHUNK_SIZE_PX as f64;
			let center_y = cursor_pos.y as f64 / limits::CHUNK_SIZE_PX as f64;

			let mut closest_position = IVec2 { x: 0, y: 0 };
			let mut closest_distance: f64 = f64::MAX;

			for ch in &chunks_to_load {
				let distance = util::distance64(center_x, center_y, ch.x as f64, ch.y as f64);
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
				chunk.send_chunk_data_to_session(session_handle, self.queue_send.clone());
			}

			if chunks_to_load.is_empty() {
				break;
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

pub struct SessionContainer {
	pub session: SessionInstanceMutex,
}

gen_id!(SessionVec, SessionContainer, SessionVecCell, SessionHandle);
