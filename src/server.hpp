#pragma once

#include "command.hpp"
#include "util/event_queue.hpp"
#include "util/listener.hpp"
#include "ws_server.hpp"
#include <functional>
#include <map>
#include <mutex>

struct Session;
struct ChunkSystem;

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
	std::mutex mtx_log;

	std::mutex mtx_sessions;
	std::map<WsConnection *, Session *> session_map; //For fast session lookup
	std::vector<uniqptr<Session>> sessions;

	WsServer server; //Needs to be at the bottom to prevent data races

public:
	Server();
	~Server();

	void log(const char *format, ...);

	void run(u16 port);

	//Locking function, do not operate on heavy loads
	void forEverySessionExcept(Session *except, std::function<void(Session *)> callback);

	//Broadcast packet for everyone except one session (optional)
	void broadcast(const Packet &packet, Session *except = nullptr);

	//Returns monochrome brush bitmap
	BrushShape *getBrushShape(u8 size, bool filled);

	ChunkSystem *getChunkSystem();

	void shutdown();

private:
	//Remove dead sessions
	void freeRemovedSessions();

	void closeCallback(WsConnection *connection);
	void messageCallback(std::shared_ptr<WsMessage> &ws_msg);

	//Non-locking methods
	Session *createSession_nolock(WsConnection *connection);
	Session *getSession_nolock(WsConnection *connection);
	void removeSession_nolock(WsConnection *connection);
	u16 findFreeSessionID_nolock();

	struct P;
	uniqptr<P> p;
};