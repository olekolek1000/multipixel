#pragma once

#include "command.hpp"
#include "util/event_queue.hpp"
#include "util/smartptr.hpp"
#include "util/timestep.hpp"
#include "util/types.hpp"
#include <atomic>
#include <memory>
#include <mutex>
#include <queue>
#include <string>
#include <string_view>
#include <thread>

struct Server;
struct WsConnection;
struct Chunk;
struct WsMessage;

struct LinkedChunk {
	Chunk *chunk;
	u32 outside_boundary_duration = 0;
};

struct Session {
private:
	bool valid = false;
	std::atomic<bool> perform_ticks = true;
	std::atomic<bool> stopping = false;
	std::atomic<bool> stopped = false;

	Server *server;
	WsConnection *connection;
	u16 id;
	std::string nickname;

	//Cursor
	bool cursor_down = false;
	s32 cursorX, cursorY, cursorX_prev, cursorY_prev, cursorX_sent, cursorY_sent;

	//Chunk visibility boundary
	struct {
		s32 start_x, start_y, end_x, end_y;
	} boundary;

	//Number of chunks received by client
	u32 chunks_received = 0;
	//Number of chunks sent by server
	u32 chunks_sent = 0;

	Timestep step_runner;
	std::thread thr_runner;

	//Queues
	std::mutex mtx_message_queue;
	std::queue<std::shared_ptr<WsMessage>> message_queue;

	std::mutex mtx_packet_queue;
	std::queue<Packet> packet_queue;

	std::mutex mtx_linked_chunks;
	std::vector<LinkedChunk> linked_chunks;

	bool needs_boundary_test;

	EventQueue queue;

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

	bool hasStopped();
	bool isStopping();

	/// Non-blocking
	void stopRunner();

	void linkChunk(Chunk *chunk);
	void unlinkChunk(Chunk *chunk);
	bool isChunkLinked(Chunk *chunk);
	bool isChunkLinked(Int2 chunk_pos);

private:
	//Send packet with exception handler
	void sendPacket(const Packet &packet);
	bool isChunkLinked_nolock(Chunk *chunk);
	bool isChunkLinked_nolock(Int2 chunk_pos);
	void close();

	void updateCursor();

	//--------------------------------------------
	// All methods below are run in worker thread
	//--------------------------------------------

	bool processed_input_message = false;

	void runner();
	bool runner_tick();
	bool runner_processMessageQueue();
	bool runner_processPacketQueue();
	void runner_performBoundaryTest();

	void parseCommand(ClientCmd cmd, const std::string_view data);
	void parseCommandAnnounce(const std::string_view data);
	void parseCommandMessage(const std::string_view data);
	void parseCommandCursorPos(const std::string_view data);
	void parseCommandCursorDown(const std::string_view data);
	void parseCommandCursorUp(const std::string_view data);
	void parseCommandBrushSize(const std::string_view data);
	void parseCommandBrushColor(const std::string_view data);
	void parseCommandBoundary(const std::string_view data);
	void parseCommandChunksReceived(const std::string_view data);

	void kick(const char *reason);
	void kickInvalidPacket();
};