#include "server.hpp"
#include "chunk_system.hpp"
#include "command.hpp"
#include "plugin.hpp"
#include "session.hpp"
#include "util/buffer.hpp"
#include "util/mutex.hpp"
#include "ws_server.hpp"
#include <atomic>
#include <cassert>
#include <chrono>
#include <cstdio>
#include <deque>
#include <math.h>
#include <mutex>
#include <signal.h>
#include <stdarg.h>
#include <thread>
#include <time.h>

static const char *LOG_SERVER = "Server";

namespace {
	auto timer_start = std::chrono::high_resolution_clock::now();
}

u64 getMicros() {
	return std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::high_resolution_clock::now() - timer_start).count();
}

u64 getMillis() {
	return std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::high_resolution_clock::now() - timer_start).count();
}

typedef std::map<u8, uniqptr<BrushShape>> BrushShapeMap;

struct Server::P {
	uniqptr<ChunkSystem> chunk_system;
	uniqptr<PluginManager> plugin_manager;

	Mutex mtx_brush_shapes;
	BrushShapeMap brush_shapes_circle_filled;
	BrushShapeMap brush_shapes_circle_outline;

	bool properly_shut_down = false;
};

Server::Server() {
	p.create();
	p->chunk_system.create(this);
	p->plugin_manager.create(this);
}

Server::~Server() {
	p->plugin_manager.reset();

	if(!p->properly_shut_down) {
		log(LOG_SERVER, "Server not properly shutted down");
	}

	log(LOG_SERVER, "Cleaning up");
}

static bool got_sigint = false;
void sigint_handler(int num) {
	if(got_sigint) {
		printf("Got more than 1 SIGINT, Hard-killing server.\n");
		exit(-1);
	} else {
		got_sigint = true;
		printf("Got SIGINT\n");
	}
}

void Server::run(u16 port) {
	signal(SIGINT, sigint_handler);

	log(LOG_SERVER, "Starting server on port %u", port);

	Mutex mtx_action;

	server.run(
			port,
			[&](std::shared_ptr<WsMessage> ws_msg) {
				LockGuard lock(mtx_action);
				messageCallback(ws_msg);
			}, //Message callback
			[&](SharedWsConnection &con) {
				LockGuard lock(mtx_action);
				closeCallback(con);
			} //Close callback
	);

	while(!got_sigint) {
		freeRemovedSessions();
		if(queue.size() > 0) {
			LockGuard lock(mtx_action);
			queue.process();
		} else {
			std::this_thread::sleep_for(std::chrono::milliseconds(20));
		}
	}

	//Clean shutdown
	shutdown();
}

void Server::shutdown() {
	log(LOG_SERVER, "======== SHUTTING DOWN SERVER ========");

	log(LOG_SERVER, "Disconnecting and removing sessions");
	LockGuard lock(mtx_sessions);
	while(!sessions.empty()) {
		log(LOG_SERVER, "%u remaining", sessions.size());
		auto *session = sessions.back().get();
		removeSession_nolock(session->getConnection().get());
	}

	p->properly_shut_down = true;
}

u16 Server::findFreeSessionID_nolock() {
	u16 id = 0;
	bool ok = false;

	while(!ok) {
		ok = true;
		for(auto &session : sessions) {
			if(session->getID() == id) {
				id++;
				ok = false;
				break;
			}
		}
	}

	return id;
}

Session *Server::createSession_nolock(SharedWsConnection &connection) {
	auto id = findFreeSessionID_nolock();

	auto &session = sessions.emplace_back();
	session.create(this, connection, id);

	auto *ptr = session.get();
	session_map_conn[connection.get()] = ptr;
	session_map_id[id] = ptr;

	log(LOG_SERVER, "Created session with ID %u (IP: %s)", id, connection->getIP());

	return ptr;
}

Session *Server::getSession_nolock(WsConnection *connection) {
	auto it = session_map_conn.find(connection);
	if(it == session_map_conn.end())
		return nullptr;

	return it->second;
}

Session *Server::getSession_nolock(u16 session_id) {
	auto it = session_map_id.find(session_id);
	if(it == session_map_id.end())
		return nullptr;

	return it->second;
}

void Server::removeSession_nolock(WsConnection *connection) {
	queue.process();

	//Remove from map
	{
		auto it = session_map_conn.find(connection);
		assert(it != session_map_conn.end());
		session_map_conn.erase(it);
	}

	//Remove from vector
	for(auto it = sessions.begin(); it != sessions.end();) {
		if((*it)->getConnection().get() != connection) {
			it++;
			continue;
		}
		auto *session_to_remove = it->get();

		getPluginManager()->passUserLeave(session_to_remove->getID());

		{
			auto it = session_map_id.find(session_to_remove->getID());
			assert(it != session_map_id.end());
			session_map_id.erase(it);
		}

		auto packet_remove_user = preparePacketUserRemove(it->get());

		//Send remove_user packet for every session (except this)
		for(auto &session : sessions) {
			if(session.get() == session_to_remove)
				continue;

			session->pushPacket(packet_remove_user);
		}

		log(LOG_SERVER, "Removing session with ID %u (Nickname %s)",
				session_to_remove->getID(),
				session_to_remove->getNickname().c_str());

		log(LOG_SERVER, "Triggering session_remove dispatchers");

		//Trigger session remove dispatcher
		dispatcher_session_remove.triggerAll(session_to_remove);

		log(LOG_SERVER, "Freeing session from memory");
		it = sessions.erase(it);
		log(LOG_SERVER, "Session freed");
		return;
	}

	//Shouldn't go there
	assert(false);
}

void Server::freeRemovedSessions() {
	LockGuard lock(mtx_sessions);

	uniqdata<Session *> to_remove;

	for(auto &session : sessions) {
		if(session->hasStopped()) {
			to_remove.push_back(session.get());
			break;
		}
	}

	for(size_t i = 0; i < to_remove.size(); i++) {
		auto *session = to_remove[i];
		removeSession_nolock(session->getConnection().get());
	}
}

void Server::forEverySessionExcept(Session *except, std::function<void(Session *)> callback) {
	LockGuard lock(mtx_sessions);

	//For every session
	for(auto &session : sessions) {
		if(session.get() == except)
			continue;

		if(!session->isValid() || session->isStopping() || session->hasStopped())
			continue;

		callback(session.get());
	}
}

void Server::broadcast_nolock(const Packet &packet, Session *except) {
	//For every session
	for(auto &session : sessions) {
		if(session.get() == except)
			continue;

		if(!session->isValid())
			continue;

		session->pushPacket(packet);
	}
}

void Server::broadcast(const Packet &packet, Session *except) {
	LockGuard lock(mtx_sessions);
	broadcast_nolock(packet, except);
}

BrushShape *Server::getBrushShape(u8 size, bool filled) {
	LockGuard lock(p->mtx_brush_shapes);

	BrushShapeMap *map;
	if(filled)
		map = &p->brush_shapes_circle_filled;
	else
		map = &p->brush_shapes_circle_outline;

	auto it = map->find(size);
	if(it == map->end()) {
		//Brush shapes doesn't exist, generate it
		auto &shape = (*map)[size];
		shape.create();
		shape->size = size;
		shape->shape.resize(size * size);
		auto *data = shape->shape.data();

		//Generate circle
		int centerX = size / 2;
		int centerY = size / 2;
		for(int y = 0; y < size; y++) {
			for(int x = 0; x < size; x++) {
				int diffX = centerX - x;
				int diffY = centerY - y;
				float distance = sqrtf(diffX * diffX + diffY * diffY);
				if(filled)
					data[y * size + x] = distance <= size / 2.0f;
				else
					data[y * size + x] = distance <= size / 2.0f && distance >= size / 2.0f - 2.0f;
			}
		}

		return shape.get();
	} else {
		return it->second.get();
	}
}

ChunkSystem *Server::getChunkSystem() {
	return p->chunk_system.get();
}

PluginManager *Server::getPluginManager() {
	return p->plugin_manager.get();
}

void Server::messageCallback(std::shared_ptr<WsMessage> &ws_msg) {
	auto &connection = ws_msg->connection;

	LockGuard lock(mtx_sessions);

	auto *session = getSession_nolock(connection.get());

	if(!session)
		session = createSession_nolock(connection);

	if(session) {
		if(!session->hasStopped() && !session->isStopping()) {
			session->pushIncomingMessage(ws_msg);
		}
	}
}

void Server::closeCallback(SharedWsConnection &connection) {
	LockGuard lock(mtx_sessions);

	auto *session = getSession_nolock(connection.get());
	if(!session) {
		log(LOG_SERVER, "Got close callback, but cannot find session");
		return;
	}

	session->stopRunner();
}

#define COLOR_RED			"\x1b[31m"
#define COLOR_GREEN		"\x1b[32m"
#define COLOR_YELLOW	"\x1b[33m"
#define COLOR_BLUE		"\x1b[34m"
#define COLOR_MAGENTA "\x1b[35m"
#define COLOR_CYAN		"\x1b[36m"
#define COLOR_RESET		"\x1b[0m"

void Server::log(const char *name, const char *format, ...) {
	LockGuard lock(mtx_log);

	char *buf = nullptr;

	va_list arglist;
	va_start(arglist, format);
	vasprintf(&buf, format, arglist);
	va_end(arglist);

	time_t time_s;
	time(&time_s);

	auto *t = localtime(&time_s);

	printf(COLOR_BLUE "[%d-%02d-%02d %02d:%02d:%02d]" COLOR_YELLOW "[%s]" COLOR_RESET " %s\n", t->tm_year + 1900, t->tm_mon + 1, t->tm_mday, t->tm_hour, t->tm_min, t->tm_sec, name, buf);
	free(buf);
}
