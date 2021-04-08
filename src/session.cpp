#include "session.hpp"
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
			cursorY_sent(0) {
	thr_runner = std::thread(&Session::runner, this);
}

Session::~Session() {
	remove = true;
	if(thr_runner.joinable())
		thr_runner.join();
}

void Session::runner() {
	step_runner.reset();
	step_runner.setRate(20);

	while(!remove) {
		bool idle = true;

		if(runner_processMessageQueue())
			idle = false;

		if(runner_processPacketQueue())
			idle = false;

		if(runner_tick())
			idle = false;

		if(idle)
			std::this_thread::sleep_for(std::chrono::milliseconds(2));
	}
}

bool Session::runner_tick() {
	if(step_runner.onTick()) {
		if(cursorX_sent != cursorX || cursorY_sent != cursorY) {
			cursorX_sent = cursorX;
			cursorY_sent = cursorY;
			server->broadcast(preparePacketUserCursorPos(getID(), cursorX, cursorY));
		}
		return true;
	}
	return false;
}

bool Session::runner_processMessageQueue() {
	mtx_message_queue.lock();
	if(!message_queue.empty()) {
		//Grab next incoming message
		auto msg = message_queue.front();
		message_queue.pop();
		mtx_message_queue.unlock();

		//Command ID
		auto command = (ClientCmd)frombig16(*(u16 *)msg->data.data());

		//Content without command (header)
		std::string_view content(msg->data.data() + sizeof(ClientCmd), msg->data.size() - sizeof(ClientCmd));

		try {
			parseCommand(command, content);
		} catch(std::exception &e) {
			server->log("Session parseCommand() failure (ID %u): %s", getID(), e.what());
		}

		return true;
	} else {
		mtx_message_queue.unlock();
		return false;
	}
}

bool Session::runner_processPacketQueue() {
	//Process packet queue
	mtx_packet_queue.lock();
	if(!packet_queue.empty()) {
		//Grab next packet
		auto packet = packet_queue.front();
		packet_queue.pop();
		mtx_packet_queue.unlock();

		//Send packet to client
		sendPacket(packet);

		return true;
	} else {
		mtx_packet_queue.unlock();
		return false;
	}
}

u16 Session::getID() {
	return this->id;
}

void Session::getMousePosition(s32 *mouseX, s32 *mouseY) {
	*mouseX = this->cursorX;
	*mouseY = this->cursorY;
}

void Session::pushIncomingMessage(std::shared_ptr<WsMessage> &msg) {
	LockGuard lock(mtx_message_queue);
	message_queue.push(msg);
}

void Session::pushPacket(const Packet &packet) {
	LockGuard lock(mtx_packet_queue);
	packet_queue.push(packet);
}

bool Session::wantsBeRemoved() {
	return remove;
}

void Session::kick(const char *reason) {
	sendPacket(preparePacket(ServerCmd::kick, reason, strlen(reason)));
	remove = true;
}

void Session::kickInvalidPacket() {
	kick("Invalid packet");
}

void Session::sendPacket(const Packet &packet) {
	try {
		getConnection()->send(packet->data(), packet->size());
	} catch(std::exception &e) {
		server->log("Session send() failure (ID %u): %s", getID(), e.what());
		remove = true;
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
		case ClientCmd::brush_size: {
			parseCommandBrushSize(data);
			break;
		}
		case ClientCmd::brush_color: {
			parseCommandBrushColor(data);
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

	brush.r = 0;
	brush.g = 0;
	brush.b = 0;
	brush.size = 1;
}

void Session::parseCommandMessage(const std::string_view data) {
	//Copy data to string
	std::string message = std::string(data);
	server->log("Got client message from IP %s: %s", connection->getIP(), message.c_str());
}

void Session::updateCursor() {
	if(!cursor_down)
		return; //Cursor is not down, do nothing

	//Draw line
	u32 iters = VecDistance({cursorX_prev, cursorY_prev}, {cursorX, cursorY});
	if(iters == 0)
		iters = 1;

	if(iters > 5000) //Too much pixels at once
		return;

	auto *brush_shape_outline = server->getBrushShape(brush.size, false);
	auto *brush_shape_filled = server->getBrushShape(brush.size, true);

	std::vector<PixelCell> pixels;
	pixels.reserve(256);

	auto addPixel = [&](s32 x, s32 y, u8 r, u8 g, u8 b) {
		auto &cell = pixels.emplace_back();
		cell.x = x;
		cell.y = y;
		cell.r = r;
		cell.g = g;
		cell.b = b;
	};

	for(u32 i = 0; i <= iters; i++) {
		float alpha = i / float(iters);

		//Lerp
		s32 x = lerp(alpha, cursorX_prev, cursorX);
		s32 y = lerp(alpha, cursorY_prev, cursorY);

		switch(brush.size) {
			case 1: {
				addPixel(x, y, brush.r, brush.g, brush.b);
				break;
			}
			case 2: {
				addPixel(x, y, brush.r, brush.g, brush.b);
				addPixel(x - 1, y, brush.r, brush.g, brush.b);
				addPixel(x + 1, y, brush.r, brush.g, brush.b);
				addPixel(x, y - 1, brush.r, brush.g, brush.b);
				addPixel(x, y + 1, brush.r, brush.g, brush.b);
				break;
			}
			default: {
				auto *shape = i == 0 ? brush_shape_filled : brush_shape_outline;
				auto *data = shape->shape.data();
				for(int yy = 0; yy < shape->size; yy++) {
					for(int xx = 0; xx < shape->size; xx++) {
						if(data[yy * shape->size + xx]) {
							addPixel(x + xx - brush.size / 2, y + yy - brush.size / 2, brush.r, brush.g, brush.b);
						}
					}
				}
				break;
			}
		}
	}

	server->setPixels(pixels.data(), pixels.size());
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
	cursorX_prev = cursorX;
	cursorY_prev = cursorY;
	updateCursor();
}

void Session::parseCommandCursorUp(const std::string_view data) {
	cursor_down = false;
	updateCursor();
}

void Session::parseCommandBrushSize(const std::string_view data) {
	if(data.size() != 1) {
		kickInvalidPacket();
		return;
	}

	auto size = *(uint8_t *)data.data();
	if(size < 1 || size > 15) {
		kickInvalidPacket();
	}

	brush.size = size;
}

void Session::parseCommandBrushColor(const std::string_view data) {
	struct PACKED {
		u8 r, g, b;
	} rgb;

	if(data.size() != sizeof(rgb)) {
		kickInvalidPacket();
		return;
	}

	memcpy(&rgb, data.data(), data.size());

	brush.r = rgb.r;
	brush.g = rgb.g;
	brush.b = rgb.b;
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