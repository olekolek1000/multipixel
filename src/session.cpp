#include "session.hpp"
#include "chunk.hpp"
#include "chunk_system.hpp"
#include "command.hpp"
#include "server.hpp"
#include "util/timestep.hpp"
#include "util/types.hpp"
#include "ws_server.hpp"
#include <array>
#include <math.h>

Session::Session(Server *server, WsConnection *connection, u16 id)
		: server(server),
			connection(connection),
			id(id),
			cursorX(0),
			cursorY(0),
			cursorX_prev(0),
			cursorY_prev(0),
			cursorX_sent(0),
			cursorY_sent(0),
			needs_boundary_test(false) {
	thr_runner = std::thread(&Session::runner, this);
}

Session::~Session() {
	stopRunner();

	if(thr_runner.joinable())
		thr_runner.join();

	while(!linked_chunks.empty()) {
		fprintf(stderr, "Session linked chunks NOT empty");
		abort();
	}
}

void Session::runner() {
	step_runner.reset();
	step_runner.setRate(20);

	while(perform_ticks) {
		bool idle = true;
		processed_input_message = false;

		if(runner_processMessageQueue()) {
			processed_input_message = true;
			idle = false;
		}

		if(runner_processPacketQueue())
			idle = false;

		if(runner_tick())
			idle = false;

		if(queue.process(1))
			idle = false;

		if(idle)
			std::this_thread::sleep_for(std::chrono::milliseconds(2));
	}

	stopped = true;
	stopping = false;
}

bool Session::runner_tick() {
	while(step_runner.onTick()) {
		auto ticks = step_runner.getTicks();

		if(cursorX_sent != cursorX || cursorY_sent != cursorY) {
			cursorX_sent = cursorX;
			cursorY_sent = cursorY;
			server->broadcast(preparePacketUserCursorPos(getID(), cursorX, cursorY));
		}

		//Every 1s
		if(ticks % 20 == 0) {
			//Remove chunks outside bounds and left for longer time
			std::vector<Int2> chunks_to_unload;
			{
				std::lock_guard lock(mtx_access);
				for(size_t i = 0; i < linked_chunks.size(); i++) { //Do not use iterator there
					auto &linked_chunk = linked_chunks[i];
					auto pos = linked_chunk.chunk->getPosition();
					if(pos.y < boundary.start_y || pos.y > boundary.end_y || pos.x < boundary.start_x || pos.x > boundary.end_x) {
						linked_chunk.outside_boundary_duration++;
						if(linked_chunk.outside_boundary_duration == 20 /* seconds */) {
							chunks_to_unload.push_back(pos);
						}
					} else {
						linked_chunk.outside_boundary_duration = 0;
					}
				}
			}

			//Deannounce chunks from list
			for(auto &pos : chunks_to_unload) {
				server->getChunkSystem()->deannounceChunkForSession(this, pos);
			}
		}

		if(tool.type == ToolType::floodfill && cursor_down && !floodfill.stack.empty()) {
			u8 r, g, b;

			auto checkColor = [&](s32 x, s32 y) {
				if(!getPixelGlobal_nolock({x, y}, &r, &g, &b)) {
					return false;
				}

				if(r == tool.r &&
					 g == tool.g &&
					 b == tool.b) {
					return false;
				}

				if(r != floodfill.to_replace_r ||
					 g != floodfill.to_replace_g ||
					 b != floodfill.to_replace_b) {
					return false;
				}

				return true;
			};

			for(u32 i = 0; i < 20000;) {
				std::lock_guard lock(mtx_access); //Required by getPixelGlobal_nolock

				if(floodfill.stack.empty()) break;
				auto cell = floodfill.stack.top();
				floodfill.stack.pop();

				if(abs(floodfill.start_x - cell.x) + abs(floodfill.start_y - cell.y) > 600) {
					continue;
				}

				setPixelQueued_nolock({cell.x, cell.y}, tool.r, tool.g, tool.b);

				bool added = false;

				if(checkColor(cell.x, cell.y - 1)) {
					auto &top = floodfill.stack.emplace();
					top.x = cell.x;
					top.y = cell.y - 1;
					added = true;
				}

				if(checkColor(cell.x - 1, cell.y)) {
					auto &left = floodfill.stack.emplace();
					left.x = cell.x - 1;
					left.y = cell.y;
					added = true;
				}

				if(checkColor(cell.x, cell.y + 1)) {
					auto &bottom = floodfill.stack.emplace();
					bottom.x = cell.x;
					bottom.y = cell.y + 1;
					added = true;
				}

				if(checkColor(cell.x + 1, cell.y)) {
					auto &right = floodfill.stack.emplace();
					right.x = cell.x + 1;
					right.y = cell.y;
					added = true;
				}

				if(added) {
					i++;
				}
			}
		}

		runner_performBoundaryTest();
		return true;
	}
	return false;
}

bool Session::runner_processMessageQueue() {
	mtx_message_queue.lock();
	if(message_queue.empty()) {
		mtx_message_queue.unlock();
		return false;
	}

	//Grab next incoming message
	auto msg = message_queue.front();
	message_queue.pop();
	mtx_message_queue.unlock();

	if(msg->data.size() < sizeof(ClientCmd)) {
		kickInvalidPacket();
		return false;
	}

	//Command ID
	u16 command_BE;
	memcpy(&command_BE, msg->data.data(), sizeof(u16));
	auto command = (ClientCmd)frombig16(command_BE);

	//Content without command (header)
	std::string_view content(msg->data.data() + sizeof(ClientCmd), msg->data.size() - sizeof(ClientCmd));

	try {
		parseCommand(command, content);
	} catch(std::exception &e) {
		server->log("Session parseCommand() failure (ID %u): %s", getID(), e.what());
	}

	return true;
}

bool Session::runner_processPacketQueue() {
	mtx_packet_queue.lock();
	if(packet_queue.empty()) {
		mtx_packet_queue.unlock();
		return false;
	}

	//Grab next packet
	auto packet = packet_queue.front();
	packet_queue.pop();
	mtx_packet_queue.unlock();

	//Send packet to client
	sendPacket(packet);

	return true;
}

u16 Session::getID() {
	return this->id;
}

void Session::getMousePosition(s32 *mouseX, s32 *mouseY) {
	*mouseX = this->cursorX;
	*mouseY = this->cursorY;
}

void Session::pushIncomingMessage(std::shared_ptr<WsMessage> &msg) {
	std::lock_guard lock(mtx_message_queue);
	message_queue.push(msg);
}

void Session::pushPacket(const Packet &packet) {
	std::lock_guard lock(mtx_packet_queue);
	packet_queue.push(packet);
}

bool Session::hasStopped() {
	return stopped;
}

bool Session::isStopping() {
	return stopping;
}

void Session::stopRunner() {
	if(!stopping) {
		stopping = true;
		perform_ticks = false;
	}
}

void Session::linkChunk(Chunk *chunk) {
	std::lock_guard lock(mtx_access);

	for(auto &cell : linked_chunks) {
		if(cell.chunk == chunk)
			return; //Already linked
	}

	pushPacket(preparePacketChunkCreate(chunk->getPosition()));

	auto &linked_chunk = linked_chunks.emplace_back();
	linked_chunk.chunk = chunk;
}

void Session::unlinkChunk(Chunk *chunk) {
	std::lock_guard lock(mtx_access);

	if(last_accessed_chunk_cache == chunk)
		last_accessed_chunk_cache = nullptr;

	for(auto it = linked_chunks.begin(); it != linked_chunks.end();) {
		if(it->chunk == chunk) {
			//Remove linked chunk
			pushPacket(preparePacketChunkRemove(chunk->getPosition()));
			it = linked_chunks.erase(it);
			return;
		} else {
			it++;
		}
	}
}

bool Session::isChunkLinked(Chunk *chunk) {
	std::lock_guard lock(mtx_access);
	return isChunkLinked_nolock(chunk);
}

bool Session::isChunkLinked(Int2 chunk_pos) {
	std::lock_guard lock(mtx_access);
	return isChunkLinked_nolock(chunk_pos);
}

bool Session::isChunkLinked_nolock(Chunk *chunk) {
	for(auto &cell : linked_chunks) {
		if(cell.chunk == chunk)
			return true;
	}
	return false;
}

bool Session::isChunkLinked_nolock(Int2 chunk_pos) {
	for(auto &cell : linked_chunks) {
		if(cell.chunk->getPosition() == chunk_pos)
			return true;
	}
	return false;
}

Chunk *Session::getChunkCached_nolock(Int2 chunk_pos) {
	if(last_accessed_chunk_cache && last_accessed_chunk_cache->getPosition() == chunk_pos)
		return last_accessed_chunk_cache;

	for(auto &chunk : linked_chunks) {
		if(chunk.chunk->getPosition() == chunk_pos) {
			last_accessed_chunk_cache = chunk.chunk;
			return chunk.chunk;
		}
	}

	return nullptr;
}

bool Session::getPixelGlobal_nolock(Int2 global_pos, u8 *r, u8 *g, u8 *b) {
	auto chunk_pos = ChunkSystem::globalPixelPosToChunkPos(global_pos);
	auto *chunk = getChunkCached_nolock(chunk_pos);
	if(!chunk)
		return false;

	auto local_pos = ChunkSystem::globalPixelPosToLocalPixelPos(global_pos);
	chunk->lock();
	chunk->allocateImage_nolock();
	chunk->getPixel_nolock(local_pos, r, g, b);
	chunk->unlock();

	return true;
}

void Session::setPixelsGlobal(GlobalPixel *pixels, size_t count) {
	std::lock_guard lock(mtx_access);
	setPixelsGlobal_nolock(pixels, count);
}

void Session::historyCreateSnapshot() {
	if(history_cells.size() > 10)
		history_cells.erase(history_cells.begin());

	history_cells.emplace_back();
}

void Session::historyUndo_nolock() {
	if(history_cells.empty())
		return; //Nothing to undo

	auto &back = history_cells.back();
	setPixelsGlobal_nolock(back.pixels.data(), back.pixels.size());

	history_cells.pop_back();
}

void Session::historyAddPixel(GlobalPixel *pixel) {
	if(history_cells.empty())
		historyCreateSnapshot();

	auto &back = history_cells.back();
	back.pixels.push_back(*pixel);
}

void Session::setPixelsGlobal_nolock(GlobalPixel *pixels, size_t count) {
	struct ChunkCacheCell {
		Int2 chunk_pos;
		Chunk *chunk;
		std::vector<ChunkPixel> queued_pixels;
		std::vector<Int2> queued_global_positions;
	};

	//Chunk "cache"
	//Visible and affected chunks by player
	std::vector<ChunkCacheCell> affected_chunks;

	auto fetchCell = [&](Int2 chunk_pos) -> ChunkCacheCell * {
		for(auto &cell : affected_chunks) {
			if(cell.chunk_pos == chunk_pos) {
				return &cell;
			}
		}

		return nullptr;
	};

	auto cacheNewChunk = [&](Int2 chunk_pos) {
		auto *chunk = getChunkCached_nolock(chunk_pos);
		if(!chunk) return;
		affected_chunks.push_back({Int2(chunk_pos), chunk});
	};

	//Generate affected chunks list
	for(size_t i = 0; i < count; i++) {
		auto &pixel = pixels[i];
		auto chunk_pos = ChunkSystem::globalPixelPosToChunkPos(pixel.pos);
		if(fetchCell(chunk_pos) == nullptr) {
			cacheNewChunk(chunk_pos);
		}
	}

	//Send pixels to chunks
	for(size_t i = 0; i < count; i++) {
		auto &pixel = pixels[i];
		auto chunk_pos = ChunkSystem::globalPixelPosToChunkPos(pixel.pos);
		auto *cell = fetchCell(chunk_pos);
		if(!cell)
			continue; //Skip pixel

		auto &queued_pixel = cell->queued_pixels.emplace_back();
		auto &queued_global_pos = cell->queued_global_positions.emplace_back();
		queued_global_pos = pixel.pos;
		queued_pixel.pos = ChunkSystem::globalPixelPosToLocalPixelPos(pixel.pos);
		queued_pixel.r = pixel.r;
		queued_pixel.g = pixel.g;
		queued_pixel.b = pixel.b;
	}

	for(auto &cell : affected_chunks) {
		if(cell.queued_pixels.empty())
			continue;

		cell.chunk->lock();
		cell.chunk->allocateImage_nolock();

		for(u32 i = 0; i < cell.queued_pixels.size(); i++) {
			auto &queued_pixel = cell.queued_pixels[i];
			auto &queued_global_pos = cell.queued_global_positions[i];

			GlobalPixel gpixel;
			gpixel.pos = queued_global_pos;
			cell.chunk->getPixel_nolock(queued_pixel.pos, &gpixel.r, &gpixel.g, &gpixel.b);
			if(gpixel.r != queued_pixel.r || gpixel.g != queued_pixel.g || gpixel.b != queued_pixel.b)
				historyAddPixel(&gpixel);
		}

		cell.chunk->setPixels_nolock(cell.queued_pixels.data(), cell.queued_pixels.size());

		cell.chunk->unlock();
	}
}

void Session::setPixelQueued_nolock(Int2 global_pos, u8 r, u8 g, u8 b) {
	auto chunk_pos = ChunkSystem::globalPixelPosToChunkPos(global_pos);
	auto *chunk = getChunkCached_nolock(chunk_pos);
	if(!chunk)
		return;

	auto local_pos = ChunkSystem::globalPixelPosToLocalPixelPos(global_pos);

	GlobalPixel global_pixel;
	global_pixel.pos = global_pos;
	chunk->lock();
	chunk->allocateImage_nolock();
	chunk->getPixel_nolock(local_pos, &global_pixel.r, &global_pixel.g, &global_pixel.b);
	chunk->unlock();
	if(global_pixel.r != r || global_pixel.g != g || global_pixel.b != b)
		historyAddPixel(&global_pixel);

	ChunkPixel pixel;
	pixel.pos = local_pos;
	pixel.r = r;
	pixel.g = g;
	pixel.b = b;
	chunk->setPixelQueued(&pixel);
}

void Session::kick(const char *reason) {
	sendPacket(preparePacket(ServerCmd::kick, reason, strlen(reason)));
	stopRunner();
}

void Session::kickInvalidPacket() {
	kick("Invalid packet");
}

void Session::sendPacket(const Packet &packet) {
	try {
		getConnection()->send(packet->data(), packet->size());
	} catch(std::exception &e) {
		server->log("Session send() failure (ID %u): %s", getID(), e.what());
		stopRunner();
	}
}

void Session::close() {
	try {
		connection->close();
	} catch(std::exception &e) {
		server->log("Session close() failure (ID %u): %s", getID(), e.what());
	}
}

void Session::parseCommand(ClientCmd cmd, const std::string_view data) {
	if(!valid && cmd != ClientCmd::announce) {
		//ClientCmd::announce should be the first message sent by client
		kick("Announcement packet expected");
		return;
	}

	switch(cmd) {
		case ClientCmd::announce: {
			parseCommandAnnounce(data);
			break;
		}
		case ClientCmd::message: {
			parseCommandMessage(data);
			break;
		}
		case ClientCmd::cursor_pos: {
			parseCommandCursorPos(data);
			break;
		}
		case ClientCmd::cursor_down: {
			parseCommandCursorDown(data);
			break;
		}
		case ClientCmd::cursor_up: {
			parseCommandCursorUp(data);
			break;
		}
		case ClientCmd::undo: {
			parseCommandUndo(data);
			break;
		}
		case ClientCmd::tool_size: {
			parseCommandToolSize(data);
			break;
		}
		case ClientCmd::tool_color: {
			parseCommandToolColor(data);
			break;
		}
		case ClientCmd::tool_type: {
			parseCommandToolType(data);
			break;
		}
		case ClientCmd::boundary: {
			parseCommandBoundary(data);
			break;
		}
		case ClientCmd::chunks_received: {
			parseCommandChunksReceived(data);
			break;
		}
		case ClientCmd::ping: {
			break;
		}
		default: {
			server->log("Got unknown command %d from IP %s (ID %u)", (int)cmd, connection->getIP(), getID());
			kick("Got unknown packet");
			break;
		}
	}
}

void Session::parseCommandAnnounce(const std::string_view data) {
	if(valid) {
		kick("Already announced");
		return;
	}

	auto nickname = std::string(data);

	if(nickname.size() < 3 || nickname.size() > 48) { //Invalid nickname length
		server->log("Client ID %u joined with invalid nickname", getID());
		this->nickname = "<invalid>";
		kick("Invalid nickname length");
		return;
	}

	this->nickname = nickname;

	server->log("Client ID %u announced as %s", getID(), getNickname().c_str());
	valid = true;

	//Send ID
	u16 id = tobig16(getID());
	sendPacket(preparePacket(ServerCmd::your_id, &id, sizeof(id)));

	//Announce all other sessions (except this session) that this session is alive
	server->broadcast(preparePacketUserCreate(this), this);

	//Init all alive sessions to this session
	server->forEverySessionExcept(this, [&](Session *other) {
		sendPacket(preparePacketUserCreate(other));
	});

	tool.r = 0;
	tool.g = 0;
	tool.b = 0;
	tool.size = 1;
	tool.type = ToolType::brush;
}

void Session::parseCommandMessage(const std::string_view data) {
	//Copy data to string
	std::string message = std::string(data);

	char buf[1024];
	snprintf(buf, sizeof(buf), "<%s> %s", getNickname().c_str(), message.c_str());

	server->log("[%s] <%s> %s", connection->getIP(), getNickname().c_str(), message.c_str());
	server->broadcast(preparePacketMessage(buf));
}

void Session::updateCursor() {
	switch(tool.type) {
		case ToolType::brush: {
			if(!cursor_down)
				break; //Cursor is not down, do nothing

			u32 iters = VecDistance({cursorX_prev, cursorY_prev}, {cursorX, cursorY});
			if(iters == 0)
				iters = 1;

			if(iters > 300) { //Too much pixels at one iteration, stop drawing (prevent griefing and server overload)
				cursor_down = false;
				break;
			}

			auto *brush_shape_outline = server->getBrushShape(tool.size, false);
			auto *brush_shape_filled = server->getBrushShape(tool.size, true);

			std::vector<GlobalPixel> pixels;
			pixels.reserve(256);

			auto addPixel = [&](s32 x, s32 y, u8 r, u8 g, u8 b) {
				auto &cell = pixels.emplace_back();
				cell.pos.x = x;
				cell.pos.y = y;
				cell.r = r;
				cell.g = g;
				cell.b = b;
			};

			//Draw line
			for(u32 i = 0; i <= iters; i++) {
				float alpha = i / float(iters);

				//Lerp
				s32 x = lerp(alpha, cursorX_prev, cursorX);
				s32 y = lerp(alpha, cursorY_prev, cursorY);

				switch(tool.size) {
					case 1: {
						addPixel(x, y, tool.r, tool.g, tool.b);
						break;
					}
					case 2: {
						addPixel(x, y, tool.r, tool.g, tool.b);
						addPixel(x - 1, y, tool.r, tool.g, tool.b);
						addPixel(x + 1, y, tool.r, tool.g, tool.b);
						addPixel(x, y - 1, tool.r, tool.g, tool.b);
						addPixel(x, y + 1, tool.r, tool.g, tool.b);
						break;
					}
					default: {
						auto *shape = i == 0 ? brush_shape_filled : brush_shape_outline;
						auto *data = shape->shape.data();
						for(int yy = 0; yy < shape->size; yy++) {
							for(int xx = 0; xx < shape->size; xx++) {
								if(data[yy * shape->size + xx]) {
									addPixel(x + xx - tool.size / 2, y + yy - tool.size / 2, tool.r, tool.g, tool.b);
								}
							}
						}
						break;
					}
				}
			}

			setPixelsGlobal(pixels.data(), pixels.size());
			break;
		}
		case ToolType::floodfill: {
			if(cursor_just_clicked) {
				floodfill.stack = {};
				u8 r, g, b;
				if(!isChunkLinked(ChunkSystem::globalPixelPosToChunkPos({cursorX, cursorY})))
					break;

				if(!getPixelGlobal_nolock({cursorX, cursorY}, &r, &g, &b))
					break;

				if(!(tool.r == r && tool.g == g && tool.b == b)) {
					floodfill.to_replace_r = r;
					floodfill.to_replace_g = g;
					floodfill.to_replace_b = b;
					floodfill.start_x = cursorX;
					floodfill.start_y = cursorY;
					auto &cell = floodfill.stack.emplace();
					cell.x = cursorX;
					cell.y = cursorY;
				}
			}
			break;
		}
	}

	cursor_just_clicked = false;
}

void Session::parseCommandCursorPos(const std::string_view data) {
	struct PACKED {
		s32 x = UINT32_MAX;
		s32 y = UINT32_MAX;
	} cursor_pos;

	if(data.size() != sizeof(cursor_pos)) {
		kickInvalidPacket();
		return;
	}

	memcpy(&cursor_pos, data.data(), data.size());

	cursor_pos.x = frombig32(cursor_pos.x);
	cursor_pos.y = frombig32(cursor_pos.y);

	this->cursorX_prev = this->cursorX;
	this->cursorY_prev = this->cursorY;
	this->cursorX = cursor_pos.x;
	this->cursorY = cursor_pos.y;

	updateCursor();
}

void Session::parseCommandCursorDown(const std::string_view data) {
	cursor_down = true;
	cursor_just_clicked = true;
	cursorX_prev = cursorX;
	cursorY_prev = cursorY;
	historyCreateSnapshot();
	updateCursor();
}

void Session::parseCommandCursorUp(const std::string_view data) {
	cursor_down = false;
	floodfill.stack = {};
	updateCursor();
}

void Session::parseCommandUndo(const std::string_view data) {
	std::lock_guard lock(mtx_access);
	historyUndo_nolock();
}

void Session::parseCommandToolSize(const std::string_view data) {
	if(data.size() != 1) {
		kickInvalidPacket();
		return;
	}

	auto size = *(uint8_t *)data.data();
	if(size < 1 || size > 8) {
		kickInvalidPacket();
		return;
	}

	tool.size = size;
}

void Session::parseCommandToolColor(const std::string_view data) {
	struct PACKED {
		u8 r, g, b;
	} rgb;

	if(data.size() != sizeof(rgb)) {
		kickInvalidPacket();
		return;
	}

	memcpy(&rgb, data.data(), data.size());

	tool.r = rgb.r;
	tool.g = rgb.g;
	tool.b = rgb.b;
}

void Session::parseCommandToolType(const std::string_view data) {
	if(data.size() != 1) {
		kickInvalidPacket();
		return;
	}

	auto type = *(uint8_t *)data.data();
	if(type < 0 || type > 1) {
		kickInvalidPacket();
		return;
	}

	tool.type = (ToolType)type;
}

void Session::parseCommandBoundary(const std::string_view data) {
	struct PACKED boundary_t {
		s32 start_x, start_y, end_x, end_y;
	};

	auto *bnd = (boundary_t *)data.data();

	if(data.size() != sizeof(boundary_t)) {
		kickInvalidPacket();
		return;
	}

	auto start_x = frombig32(bnd->start_x);
	auto start_y = frombig32(bnd->start_y);
	auto end_x = frombig32(bnd->end_x);
	auto end_y = frombig32(bnd->end_y);

	if(end_y < start_y)
		end_y = start_y;

	if(end_x < start_x)
		end_x = start_x;

	//Chunk limit
	end_x = std::min(end_x, start_x + 100);
	end_y = std::min(end_y, start_y + 100);

	boundary.start_x = start_x;
	boundary.start_y = start_y;
	boundary.end_x = end_x;
	boundary.end_y = end_y;

	needs_boundary_test = true;
}

void Session::parseCommandChunksReceived(const std::string_view data) {
	u32 chunks_received_BE;
	memcpy(&chunks_received_BE, data.data(), sizeof(u32));
	auto chunks_received = frombig32(chunks_received_BE);
	if(chunks_received <= this->chunks_received) {
		kickInvalidPacket();
		return;
	}
	this->chunks_received = chunks_received;
}

void Session::runner_performBoundaryTest() {
	if(processed_input_message)
		return; //Process new chunks only when all client input messages are read

	if(!needs_boundary_test)
		return;
	needs_boundary_test = false;

	std::vector<Int2> chunks_to_load;
	{
		std::lock_guard lock(mtx_access);

		//Check which chunks aren't announced for this session
		for(s32 y = boundary.start_y; y < boundary.end_y; y++) {
			for(s32 x = boundary.start_x; x < boundary.end_x; x++) {
				if(!isChunkLinked_nolock({x, y})) {
					chunks_to_load.push_back({x, y});
				}
			}
		}
	}

	//No chunks to load
	if(chunks_to_load.empty()) return;

	s64 in_queue = (s64)chunks_sent - (s64)chunks_received;
	s32 to_send = 40 - in_queue; //Max 40 queued chunks

	for(s32 iterations = 0; iterations < to_send; iterations++) {
		if(chunks_to_load.empty())
			break;

		//Get closest chunk (circular loading)
		float center_x = (float)cursorX / ChunkSystem::getChunkSize();
		float center_y = (float)cursorY / ChunkSystem::getChunkSize();
		Int2 closest_position;
		float closest_distance = __FLT_MAX__;
		for(auto &ch : chunks_to_load) {
			float distance = VecDistance({center_x, center_y}, {(float)ch.x, (float)ch.y});
			if(distance < closest_distance) {
				closest_distance = distance;
				closest_position = ch;
			}
		}

		//Remove closest chunks_to_load cell (mark as loaded)
		for(auto it = chunks_to_load.begin(); it != chunks_to_load.end();) {
			if(*it == closest_position) {
				it = chunks_to_load.erase(it);
			} else {
				it++;
			}
		}

		//Announce chunk
		chunks_sent++;
		server->getChunkSystem()->announceChunkForSession(this, closest_position);
	}

	needs_boundary_test = !chunks_to_load.empty();
}

bool Session::isValid() {
	return valid;
}

const std::string &Session::getNickname() {
	return nickname;
}

WsConnection *Session::getConnection() {
	return this->connection;
}