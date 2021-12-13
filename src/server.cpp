#include "server.hpp"
#include "chunk_system.hpp"
#include "command.hpp"
#include "plugin.hpp"
#include "room.hpp"
#include "session.hpp"
#include "util/buffer.hpp"
#include "util/mutex.hpp"
#include "ws_server.hpp"
#include <atomic>
#include <cassert>
#include <chrono>
#include <cstdio>
#include <deque>
#include <filesystem>
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

Server::Server() {
	// Create "Rooms" directory
	if(!std::filesystem::is_directory("rooms"))
		std::filesystem::create_directory("rooms");
}

Server::~Server() {
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
			}, // Message callback
			[&](SharedWsConnection &con) {
				LockGuard lock(mtx_action);
				closeCallback(con);
			} // Close callback
	);

	while(!got_sigint) {
		bool busy = false;
		{
			LockGuard lock(mtx_action);
			LockGuard lock2(mtx_rooms);
			for(auto &room : rooms) {
				if(room->tick())
					busy = true;
			}
		}

		if(!busy) {
			std::this_thread::sleep_for(std::chrono::milliseconds(20));
		}
	}

	// Clean shutdown
	shutdown();
}

void Server::shutdown() {
	log(LOG_SERVER, "======== SHUTTING DOWN SERVER ========");

	// Free rooms
	{
		log(LOG_SERVER, "Freeing rooms");
		rooms.clear();
	}

	// Free sessions
	{
		log(LOG_SERVER, "Disconnecting and removing sessions");
		LockGuard lock(mtx_sessions);
		while(!sessions.empty()) {
			log(LOG_SERVER, "%u remaining", sessions.size());
			auto *session = sessions.back().get();
			removeSession_nolock(session->getConnection().get());
		}
	}
}

Session *Server::createSession_nolock(SharedWsConnection &connection) {
	auto &session = sessions.emplace_back();
	session = std::make_shared<Session>(this, connection);

	auto *ptr = session.get();
	session_map_conn[connection.get()] = ptr;

	log(LOG_SERVER, "Created session (IP: %s)", connection->getIP());

	return ptr;
}

Session *Server::getSession_nolock(WsConnection *connection) {
	auto it = session_map_conn.find(connection);
	if(it == session_map_conn.end())
		return nullptr;

	return it->second;
}

void Server::removeSession(WsConnection *connection) {
	LockGuard lock(mtx_sessions);
	removeSession_nolock(connection);
}

void Server::removeSession_nolock(WsConnection *connection) {
	// Remove from map
	{
		auto it = session_map_conn.find(connection);
		assert(it != session_map_conn.end());
		session_map_conn.erase(it);
	}

	// Remove from vector
	for(auto it = sessions.begin(); it != sessions.end();) {
		if((*it)->getConnection().get() != connection) {
			it++;
			continue;
		}
		auto *session_to_remove = it->get();

		log(LOG_SERVER, "Removing session (Nickname %s)",
				session_to_remove->getNickname().c_str());

		it = sessions.erase(it);
		return;
	}

	// Shouldn't go there
	assert(false);
}

Room *Server::getOrCreateRoom(std::string_view room_name) {
	LockGuard lock(mtx_rooms);

	// Iterate all rooms
	for(auto &room : rooms) {
		if(room->getName() == room_name) {
			return room.get();
		}
	}

	// Create new room
	auto &room = rooms.emplace_back();
	room.create(this, room_name);
	return room.get();
}

void Server::forEverySessionExcept(Session *except, std::function<void(Session *)> callback) {
	LockGuard lock(mtx_sessions);

	// For every session
	for(auto &session : sessions) {
		if(session.get() == except)
			continue;

		if(!session->isValid() || session->isStopping() || session->hasStopped())
			continue;

		callback(session.get());
	}
}

void Server::broadcastGlobal_nolock(const Packet &packet, Session *except) {
	// For every session
	for(auto &session : sessions) {
		if(session.get() == except)
			continue;

		if(!session->isValid())
			continue;

		session->pushPacket(packet);
	}
}

void Server::broadcastGlobal(const Packet &packet, Session *except) {
	LockGuard lock(mtx_sessions);
	broadcastGlobal_nolock(packet, except);
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
