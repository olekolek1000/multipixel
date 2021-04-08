#include "server.hpp"
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
	Mutex mtx_pixel_queue;
	std::atomic<bool> pixels_in_queue = false;
	std::vector<PixelCell> pixel_queue;

	Mutex mtx_brush_shapes;
	BrushShapeMap brush_shapes_circle_filled;
	BrushShapeMap brush_shapes_circle_outline;
};

Server::Server() {
	p.create();
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

		if(queue.process(10))
			idle = false;

		if(processPixelQueue())
			idle = false;

		if(idle)
			std::this_thread::sleep_for(std::chrono::milliseconds(5));
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

			log("Removing session with ID %u (IP %s, Nickname %s)", session_to_remove->getID(), session_to_remove->getConnection()->getIP(), session_to_remove->getNickname().c_str());

			//Remove session completely
			//This can hang if Session::runner thread
			//is freezed somehow (~Session() joins Session::runner thread).
			//Good luck debugging that
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

void Server::setPixels(PixelCell *cells, size_t count) {
	LockGuard lock(p->mtx_pixel_queue);
	auto prev_size = p->pixel_queue.size();
	p->pixel_queue.resize(p->pixel_queue.size() + count);
	memcpy(p->pixel_queue.data() + prev_size, cells, count * sizeof(PixelCell));
	p->pixels_in_queue = true;
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

bool Server::processPixelQueue() {
	bool expect = true;
	if(!p->pixels_in_queue.compare_exchange_weak(expect, false))
		return false; //pixels_in_queue not set, return

	Buffer buf_pixels;
	size_t pixel_count = 0;

	{
		LockGuard lock(p->mtx_pixel_queue);
		pixel_count = p->pixel_queue.size();

		const size_t x_size = sizeof(s32), y_size = sizeof(s32),
								 r_size = sizeof(u8), g_size = sizeof(u8), b_size = sizeof(u8);

		const size_t pixel_size = x_size + y_size + r_size + g_size + b_size;
		buf_pixels.reserve(pixel_count * pixel_size);

		auto *data = p->pixel_queue.data();

		//Write X
		for(u32 i = 0; i < pixel_count; i++) {
			s32 x_BE = tobig32(data[i].x);
			buf_pixels.write(&x_BE, sizeof(s32));
		}

		//Write Y
		for(u32 i = 0; i < pixel_count; i++) {
			s32 y_BE = tobig32(data[i].y);
			buf_pixels.write(&y_BE, sizeof(s32));
		}

		//Write R
		for(u32 i = 0; i < pixel_count; i++) {
			u8 red = data[i].r;
			buf_pixels.write(&red, sizeof(u8));
		}

		//Write G
		for(u32 i = 0; i < pixel_count; i++) {
			u8 green = data[i].g;
			buf_pixels.write(&green, sizeof(u8));
		}

		//Write B
		for(u32 i = 0; i < pixel_count; i++) {
			u8 blue = data[i].b;
			buf_pixels.write(&blue, sizeof(u8));
		}

		//Batch erase
		p->pixel_queue.erase(p->pixel_queue.begin(), p->pixel_queue.begin() + pixel_count);
	}

	auto compressed = compressLZ4(buf_pixels.data(), buf_pixels.size());

	u32 count_BE = tobig32((u32)pixel_count);
	u32 raw_size_BE = tobig32((u32)buf_pixels.size());
	u32 compressed_size_BE = tobig32((u32)compressed.size());

	Datasize data_pixel_count(&count_BE, sizeof(u32));
	Datasize data_raw_size(&raw_size_BE, sizeof(u32));
	Datasize data_compressed_size(&compressed_size_BE, sizeof(u32));
	Datasize data_compressed_data(compressed.data(), compressed.size());

	Datasize *datasizes[] =
			{&data_pixel_count,
			 &data_raw_size,
			 &data_compressed_size,
			 &data_compressed_data,
			 nullptr};

	auto packet = preparePacket(ServerCmd::pixel_pack, datasizes);

	broadcast(packet);

	return true;
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
