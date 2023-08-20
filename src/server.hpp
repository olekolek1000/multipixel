#pragma once

#include "command.hpp"
#include "util/event_queue.hpp"
#include "util/listener.hpp"
#include "util/mutex.hpp"
#include "ws_server.hpp"
#include <functional>
#include <map>
#include <memory>
#include <mutex>
#include <set>

#define COLOR_RED			"\x1b[31m"
#define COLOR_GREEN		"\x1b[32m"
#define COLOR_YELLOW	"\x1b[33m"
#define COLOR_BLUE		"\x1b[34m"
#define COLOR_MAGENTA "\x1b[35m"
#define COLOR_CYAN		"\x1b[36m"
#define COLOR_RESET		"\x1b[0m"

struct Session;
struct ChunkSystem;
struct Room;
struct PluginManager;

u64 getMicros(); // Microseconds
u64 getMillis(); // Milliseconds

struct Server {
	WsServer server; // Needs to be at the bottom to prevent data races

private:
	std::map<WsConnection *, Session *> session_map_conn; // For fast session lookup
	std::vector<std::shared_ptr<Session>> sessions;
	std::vector<uniqptr<Room>> rooms;
	std::set<Room *> rooms_to_remove;

public:
	Mutex mtx_sessions;
	Mutex mtx_log;
	Mutex mtx_rooms;
	Mutex mtx_rooms_removal;

	Server();
	~Server();

	void log(const char *name, const char *format, ...);
	void run(u16 port);
	void shutdown();

	void forEverySessionExcept(Session *except, std::function<void(Session *)> callback);
	void broadcastGlobal(const Packet &packet, Session *except = nullptr);
	void broadcastGlobal_nolock(const Packet &packet, Session *except = nullptr);

	void removeSession(WsConnection *connection);

	Room *getOrCreateRoom(std::string_view room_name);
	void markRoomForRemoval(Room *room);

private:
	void removeRoom_nolock(Room *room);

	void closeCallback(SharedWsConnection &connection);
	void messageCallback(std::shared_ptr<WsMessage> &ws_msg);

	// Non-locking methods
	Session *createSession_nolock(SharedWsConnection &connection);
	void removeSession_nolock(WsConnection *connection);

	Session *getSession_nolock(WsConnection *connection);
};