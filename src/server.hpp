#pragma once

#include "command.hpp"
#include "util/event_queue.hpp"
#include "util/mutex.hpp"
#include "ws_server.hpp"
#include <functional>
#include <map>

struct Session;
struct ChunkSystem;

u64 getMicros(); //Microseconds
u64 getMillis(); //Milliseconds

struct BrushShape {
	u8 size; //Width and height
	uniqdata<u8> shape;
};

struct Server {
private:
	struct P;
	uniqptr<P> p;

	WsServer server;

	Mutex mtx_log;

	Mutex mtx_sessions;
	std::map<WsConnection *, Session *> session_map; //For fast session lookup
	std::vector<uniqptr<Session>> sessions;

public:
	EventQueue queue;

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

private:
	void closeCallback(WsConnection *connection);
	void messageCallback(std::shared_ptr<WsMessage> &ws_msg);

	//Non-locking methods
	Session *createSession_nolock(WsConnection *connection);
	Session *getSession_nolock(WsConnection *connection);
	void removeSession_nolock(WsConnection *connection);
	u16 findFreeSessionID_nolock();
};