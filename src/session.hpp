#pragma once

#include "command.hpp"
#include "util/mutex.hpp"
#include "util/smartptr.hpp"
#include "util/timestep.hpp"
#include "util/types.hpp"
#include <memory>
#include <queue>
#include <string>
#include <string_view>
#include <thread>

struct Server;
struct WsConnection;
struct Chunk;
struct WsMessage;

struct Session {
private:
	bool valid = false;
	bool remove = false;

	Server *server;
	WsConnection *connection;
	u16 id;
	std::string nickname;

	//Cursor
	bool cursor_down = false;
	s32 cursorX, cursorY, cursorX_prev, cursorY_prev, cursorX_sent, cursorY_sent;

	Timestep step_runner;
	std::thread thr_runner;

	//Queues
	Mutex mtx_message_queue;
	std::queue<std::shared_ptr<WsMessage>> message_queue;

	Mutex mtx_packet_queue;
	std::queue<Packet> packet_queue;

	Mutex mtx_linked_chunks;
	std::vector<Chunk *> linked_chunks;

	//Brush settings
	struct {
		u8 size;
		u8 r, g, b;
	} brush;

public:
	Session(Server *server, WsConnection *connection, u16 id);
	~Session();

	u16 getID();
	const std::string &getNickname();
	bool isValid();

	WsConnection *getConnection();
	void getMousePosition(s32 *mouseX, s32 *mouseY);

	void pushIncomingMessage(std::shared_ptr<WsMessage> &msg);
	void pushPacket(const Packet &packet);

	bool wantsBeRemoved();

	void linkChunk(Chunk *chunk);
	void unlinkChunk(Chunk *chunk);
	bool isChunkLinked(Chunk *chunk);
	bool isChunkLinked(Int2 chunk_pos);

private:
	//Send packet with exception handler
	void sendPacket(const Packet &packet);
	void close();

	void updateCursor();

	//--------------------------------------------
	// All methods below are run in worker thread
	//--------------------------------------------

	void runner();
	bool runner_tick();
	bool runner_processMessageQueue();
	bool runner_processPacketQueue();

	void parseCommand(ClientCmd cmd, const std::string_view data);
	void parseCommandAnnounce(const std::string_view data);
	void parseCommandMessage(const std::string_view data);
	void parseCommandCursorPos(const std::string_view data);
	void parseCommandCursorDown(const std::string_view data);
	void parseCommandCursorUp(const std::string_view data);
	void parseCommandBrushSize(const std::string_view data);
	void parseCommandBrushColor(const std::string_view data);

	void kick(const char *reason);
	void kickInvalidPacket();
};