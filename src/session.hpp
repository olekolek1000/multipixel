#pragma once

#include "color.hpp"
#include "command.hpp"
#include "src/waiter.hpp"
#include "util/event_queue.hpp"
#include "util/mutex.hpp"
#include "util/optional.hpp"
#include "util/smartptr.hpp"
#include "util/timestep.hpp"
#include "util/types.hpp"
#include "ws_server.hpp"
#include <atomic>
#include <memory>
#include <mutex>
#include <queue>
#include <set>
#include <stack>
#include <string>
#include <string_view>
#include <thread>
#include <unordered_set>

struct Server;
struct Chunk;
struct WsMessage;
struct Room;

struct LinkedChunk {
	Chunk *chunk;
	u32 outside_boundary_duration = 0;
};

struct FloodfillCell {
	s32 x;
	s32 y;
};

struct GlobalPixel {
	Int2 pos;
	Color color;
};

struct HistoryCell {
	std::vector<GlobalPixel> pixels;
};

struct Session : std::enable_shared_from_this<Session> {
private:
	std::atomic<bool> valid = false;
	std::atomic<bool> perform_ticks = true;
	std::atomic<bool> stopping = false;
	std::atomic<bool> stopped = false;

	Server *server;
	SharedWsConnection connection;
	Optional<SessionID> id;
	std::string nickname;

	Room *room = nullptr;

	Chunk *last_accessed_chunk_cache = nullptr;

	// Cursor
	bool cursor_down = false;
	bool cursor_just_clicked = false;
	std::atomic<Int2> cursor_pos;
	std::atomic<Int2> cursor_pos_prev;
	std::atomic<Int2> cursor_pos_sent;

	// Chunk visibility boundary
	struct {
		s32 start_x, start_y, end_x, end_y;
		float zoom;
	} boundary;

	// Number of chunks received by client
	u32 chunks_received = 0;
	// Number of chunks sent by server
	u32 chunks_sent = 0;

	Timestep step_runner;
	std::thread thr_runner;

	// Queues
	Mutex mtx_message_queue;
	std::queue<std::shared_ptr<WsMessage>> message_queue;

	Mutex mtx_packet_queue;
	std::queue<Packet> packet_queue;

	Mutex mtx_access;
	std::vector<LinkedChunk> linked_chunks;

	std::vector<HistoryCell> history_cells;

	struct {
		Color to_replace;
		std::stack<FloodfillCell> stack;
		std::set<Int2> affected_chunks;
		bool processing = false;
		s32 start_x;
		s32 start_y;
		u32 processed_count = 0;
		void reset() {
			processing = false;
			affected_chunks = {};
			stack = {};
			processed_count = 0;
		}
	} floodfill;

	bool needs_boundary_test;

	EventQueue queue;

	// Tool settings
	struct {
		u8 size;
		Color color;
		ToolType type;
	} tool;

public:
	Session(Server *server, SharedWsConnection &connection);
	~Session();

	void setID(SessionID id);
	Optional<SessionID> getID();
	const std::string &getNickname();
	bool isValid();

	SharedWsConnection &getConnection();
	void getMousePosition(s32 *mouseX, s32 *mouseY);

	// Returns queued packet count
	void pushIncomingMessage(std::shared_ptr<WsMessage> &msg);
	void pushPacket(const Packet &packet);

	bool hasStopped();
	bool isStopping();

	/// Non-blocking
	void stopRunner();
	void stopRunnerWait();

	void linkChunk(Chunk *chunk);
	void unlinkChunk(Chunk *chunk);
	bool isChunkLinked(Chunk *chunk);
	bool isChunkLinked(Int2 chunk_pos);

	Room *getRoom() const;
	inline bool hasRoom() const { return getRoom() != nullptr; }

private:
	// Send packet with exception handler
	void sendPacket(const Packet &packet);
	bool isChunkLinked_nolock(Chunk *chunk);
	bool isChunkLinked_nolock(Int2 chunk_pos);
	void close();

	void sendPacketProcessingStatusText(std::string_view text);

	//--------------------------------------------
	// All methods below are run in worker thread
	//--------------------------------------------

	bool processed_input_message = false;

	void runner();
	bool runner_tick();

	void tick_tool_floodfill();

	bool runner_processMessageQueue();
	bool runner_processPacketQueue();
	void runner_performBoundaryTest();

	void parseCommand(ClientCmd cmd, const std::string_view data);
	void parseCommandAnnounce(const std::string_view data);
	void parseCommandMessage(const std::string_view data);
	void parseCommandCursorPos(const std::string_view data);
	void parseCommandCursorDown(const std::string_view data);
	void parseCommandCursorUp(const std::string_view data);
	void parseCommandUndo(const std::string_view data);
	void parseCommandToolSize(const std::string_view data);
	void parseCommandToolColor(const std::string_view data);
	void parseCommandToolType(const std::string_view data);
	void parseCommandBoundary(const std::string_view data);
	void parseCommandChunksReceived(const std::string_view data);
	void parseCommandPreviewRequest(const std::string_view data);

	void kick(const char *reason);
	void kickInvalidPacket();

	void updateCursor();

	Chunk *getChunkCached_nolock(Int2 chunk_pos);
	bool getPixelGlobal_nolock(Int2 global_pos, Color *color);
	void setPixelQueued_nolock(Int2 global_pos, Color color);

	void setPixelsGlobal_nolock(GlobalPixel *pixels, size_t count, bool queued);
	void setPixelsGlobal(GlobalPixel *pixels, size_t count, bool queued);

	void historyCreateSnapshot();
	void historyUndo_nolock();
	void historyAddPixel(GlobalPixel *pixel);
};