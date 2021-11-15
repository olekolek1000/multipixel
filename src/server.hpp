#pragma once

#include "command.hpp"
#include "util/event_queue.hpp"
#include "util/listener.hpp"
#include "util/mutex.hpp"
#include "ws_server.hpp"
#include <functional>
#include <map>
#include <mutex>

struct Session;
struct ChunkSystem;
struct PluginManager;

u64 getMicros(); //Microseconds
u64 getMillis(); //Milliseconds

struct BrushShape {
	u8 size; //Width and height
	uniqdata<u8> shape;
};

struct Server {
public:
	MultiDispatcher<void(Session *)> dispatcher_session_remove;

private:
	Mutex mtx_log;

	std::map<WsConnection *, Session *> session_map_conn; //For fast session lookup
	std::map<u16, Session *> session_map_id;							//For fast session lookup
	std::vector<uniqptr<Session>> sessions;

	WsServer server; //Needs to be at the bottom to prevent data races

public:
	Mutex mtx_sessions;
	EventQueue queue;

	Server();
	~Server();

	void log(const char *name, const char *format, ...);

	void run(u16 port);

	//Locking function, do not operate on heavy loads
	void forEverySessionExcept(Session *except, std::function<void(Session *)> callback);

	//Broadcast packet for everyone except one session (optional)
	void broadcast(const Packet &packet, Session *except = nullptr);
	void broadcast_nolock(const Packet &packet, Session *except = nullptr);

	//Returns monochrome brush bitmap
	BrushShape *getBrushShape(u8 size, bool filled);

	ChunkSystem *getChunkSystem();
	PluginManager *getPluginManager();

	void shutdown();

	Session *getSession_nolock(WsConnection *connection);
	Session *getSession_nolock(u16 session_id);

private:
	//Remove dead sessions
	void freeRemovedSessions();

	void closeCallback(SharedWsConnection &connection);
	void messageCallback(std::shared_ptr<WsMessage> &ws_msg);

	//Non-locking methods
	Session *createSession_nolock(SharedWsConnection &connection);
	void removeSession_nolock(WsConnection *connection);
	u16 findFreeSessionID_nolock();

	struct P;
	uniqptr<P> p;
};