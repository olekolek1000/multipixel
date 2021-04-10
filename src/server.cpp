#include "server.hpp"
#include "chunk_system.hpp"
#include "command.hpp"
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
#include <signal.h>
#include <stdarg.h>
#include <thread>
#include <time.h>

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

	Mutex mtx_brush_shapes;
	BrushShapeMap brush_shapes_circle_filled;
	BrushShapeMap brush_shapes_circle_outline;
};

Server::Server() {
	p.create();
	p->chunk_system.create(this);
}

Server::~Server() {
}

static bool got_sigint = false;
void sigint_handler(int num) {
	if(got_sigint) {
		printf("Got more than 1 SIGINT, Hard-killing server. Goodbye.\n");
		exit(-1);
	} else {
		got_sigint = true;
		printf("Got SIGINT\n");
	}
}

void Server::run(u16 port) {
	signal(SIGINT, sigint_handler);

	server.run(
			port,
			[this](std::shared_ptr<WsMessage> ws_msg) { messageCallback(ws_msg); }, //Message callback
			[this](WsConnection *con) { closeCallback(con); }												//Close callback
	);

	bool idle = false;
	while(!got_sigint) {
		idle = true;

		//stonks
		if(idle)
			std::this_thread::sleep_for(std::chrono::milliseconds(10));
	}
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

Session *Server::createSession_nolock(WsConnection *connection) {
	auto id = findFreeSessionID_nolock();

	auto &session = sessions.emplace_back();
	session.create(this, connection, id);

	auto *ptr = session.get();
	session_map[connection] = ptr;

	log("Created session with ID %u (IP: %s)", id, connection->getIP());

	auto *chunk_system = getChunkSystem();
	//Announce spawn chunks (temporary)
	s32 countX = 4;
	s32 countY = 4;
	for(s32 y = -countY; y < countY; y++) {
		for(s32 x = -countX; x < countX; x++) {
			chunk_system->announceChunkForSession(ptr, {x, y});
		}
	}

	return ptr;
}

Session *Server::getSession_nolock(WsConnection *connection) {
	auto it = session_map.find(connection);
	if(it == session_map.end())
		return nullptr;

	return it->second;
}

void Server::removeSession_nolock(WsConnection *connection) {
	//Remove from map
	auto it = session_map.find(connection);
	assert(it != session_map.end());
	session_map.erase(it);

	//Remove from vector
	for(auto it = sessions.begin(); it != sessions.end();) {
		if((*it)->getConnection() == connection) {
			auto *session_to_remove = it->get();
			auto id = session_to_remove->getID();

			auto packet_remove_user = preparePacketUserRemove(it->get());

			//Send remove_user packet for every session (except this)
			for(auto &session : sessions) {
				if(session.get() == session_to_remove)
					continue;

				session->pushPacket(packet_remove_user);
			}

			log("Removing session with ID %u (IP %s, Nickname %s)",
					session_to_remove->getID(),
					session_to_remove->getConnection()->getIP(),
					session_to_remove->getNickname().c_str());

			log("Triggering session_remove dispatchers");

			log("Flushing queue");

			//Trigger session remove dispatcher
			dispatcher_session_remove.triggerAll(session_to_remove);

			//Remove session completely
			//This can hang if Session::runner thread
			//is freezed somehow (~Session() joins Session::runner thread).
			//Good luck debugging that
			log("Deallocating session");
			it = sessions.erase(it);

			log("Removed session with ID %u", id);
			return;
		} else {
			it++;
		}
	}

	//Shouldn't go there
	assert(false);
}

void Server::forEverySessionExcept(Session *except, std::function<void(Session *)> callback) {
	LockGuard lock(mtx_sessions);

	//For every session
	for(auto &session : sessions) {
		if(session.get() == except)
			continue;

		if(!session->isValid())
			continue;

		if(session->wantsBeRemoved())
			continue;

		callback(session.get());
	}
}

void Server::broadcast(const Packet &packet, Session *except) {
	LockGuard lock(mtx_sessions);

	//For every session
	for(auto &session : sessions) {
		if(session.get() == except)
			continue;

		if(!session->isValid())
			continue;

		session->pushPacket(packet);
	}
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

void Server::messageCallback(std::shared_ptr<WsMessage> &ws_msg) {
	auto *connection = ws_msg->connection;

	LockGuard lock(mtx_sessions);

	auto *session = getSession_nolock(connection);

	if(!session)
		session = createSession_nolock(connection);

	if(session) {
		if(session->wantsBeRemoved()) {
			removeSession_nolock(connection);
		} else {
			session->pushIncomingMessage(ws_msg);
		}
	}
}

void Server::closeCallback(WsConnection *connection) {
	LockGuard lock(mtx_sessions);

	auto *session = getSession_nolock(connection);
	if(!session) {
		log("Got close callback, but cannot find session");
		return;
	}

	removeSession_nolock(connection);
}

void Server::log(const char *format, ...) {
	LockGuard lock(mtx_log);

	char *buf = nullptr;

	va_list arglist;
	va_start(arglist, format);
	vasprintf(&buf, format, arglist);
	va_end(arglist);

	time_t time_s;
	time(&time_s);

	auto *t = localtime(&time_s);

	printf("[%d-%02d-%02d %02d:%02d:%02d] %s\n", t->tm_year + 1900, t->tm_mon, t->tm_mday, t->tm_hour, t->tm_min, t->tm_sec, buf);
	free(buf);
}
