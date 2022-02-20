#include "room.hpp"
#include "chunk_system.hpp"
#include "plugin.hpp"
#include "preview_system.hpp"
#include "server.hpp"
#include "src/chunk.hpp"
#include <math.h>
#include <stdarg.h>

typedef std::map<u8, uniqptr<BrushShape>> BrushShapeMap;

static const char *LOG_ROOM = "RoomManager";

struct Room::P {
	std::string name;

	uniqptr<PreviewSystem> preview_system;
	uniqptr<ChunkSystem> chunk_system;
	uniqptr<PluginManager> plugin_manager;

	Mutex mtx_brush_shapes;
	BrushShapeMap brush_shapes_circle_filled;
	BrushShapeMap brush_shapes_circle_outline;

	bool properly_shut_down = false;
};

Room::Room(Server *server, std::string_view name) {
	p.create();

	this->server = server;
	p->name = name;

	// Init database
	char db_path[256];
	snprintf(db_path, sizeof(db_path), "rooms/%s.db", getName().c_str());
	database.init(db_path);

	// Init chunk system and plugin manager
	p->chunk_system.create(this);
	p->plugin_manager.create(this);
	p->preview_system.create(this);

	database.lock();
	database.foreachChunk([this](Int2 pos) {
		p->preview_system->addToQueueFront(pos);
	});
	database.unlock();
}

Room::~Room() {
	{
		LockGuard lock(mtx_sessions);
		while(!sessions.empty()) {
			auto s = sessions.back().lock();
			if(s) {
				removeSession_nolock(s);
				server->removeSession(s->getConnection().get());
			} else {
				sessions.pop_back();
			}
		}
	}
	p->plugin_manager.reset();
	log(LOG_ROOM, "Room freed");
}

bool Room::tick() {
	getPluginManager()->passTick();
	getPreviewSystem()->tick();
	freeRemovedSessions();
	if(queue.size() > 0) {
		queue.process();
		return true;
	}
	return false;
}

const std::string &Room::getName() {
	return p->name;
}

SessionID Room::findFreeSessionID_nolock() {
	u16 id = 0;
	bool ok = false;

	while(!ok) {
		ok = true;
		for(auto &s : sessions) {
			auto session = s.lock();
			if(!session) continue;
			auto other_id = session->getID();
			if(other_id && other_id.value() == id) {
				id++;
				ok = false;
				break;
			}
		}
	}

	return id;
}

bool Room::addSession(const std::shared_ptr<Session> &session) {
	LockGuard lock(mtx_sessions);
	sessions.push_back(session);

	auto free_id = findFreeSessionID_nolock();
	session->setID(free_id);
	session_map_id[free_id] = session.get();
	log(LOG_ROOM, "Added session with ID %u", free_id.get());

	return true;
}

void Room::removeSession_nolock(const std::shared_ptr<Session> &to_remove) {
	auto opt_id = to_remove->getID();
	if(opt_id) {
		log(LOG_ROOM, "Removing session");

		auto id = opt_id.value();

		getPluginManager()->passUserLeave(id);

		auto it = session_map_id.find(id);
		assert(it != session_map_id.end());
		session_map_id.erase(it);

		auto packet_remove_user = preparePacketUserRemove(to_remove.get());

		// Send remove_user packet for every session (except this)
		for(auto &s : sessions) {
			auto session = s.lock();
			if(!session || session == to_remove)
				continue;

			session->pushPacket(packet_remove_user);
		}

		// Trigger session remove dispatcher
		log(LOG_ROOM, "Triggering session_remove dispatchers");
		dispatcher_session_remove.triggerAll(to_remove.get());
	}

	// Remove session from vector
	for(auto it = sessions.begin(); it != sessions.end();) {
		auto s = it->lock();
		if(s && s == to_remove) {
			it = sessions.erase(it);
		} else {
			it++;
		}
	}
}

void Room::removeSession(const std::shared_ptr<Session> &session) {
	LockGuard lock(mtx_sessions);
	removeSession_nolock(session);
}

Session *Room::getSession_nolock(SessionID session_id) {
	auto it = session_map_id.find(session_id);
	if(it == session_map_id.end())
		return nullptr;

	return it->second;
}

BrushShape *Room::getBrushShape(u8 size, bool filled) {
	LockGuard lock(p->mtx_brush_shapes);

	BrushShapeMap *map;
	if(filled)
		map = &p->brush_shapes_circle_filled;
	else
		map = &p->brush_shapes_circle_outline;

	auto it = map->find(size);
	if(it == map->end()) {
		// Brush shapes doesn't exist, generate it
		auto &shape = (*map)[size];
		shape.create();
		shape->size = size;
		shape->shape.resize(size * size);
		auto *data = shape->shape.data();

		// Generate circle
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

PreviewSystem *Room::getPreviewSystem() const {
	return p->preview_system.get();
}

ChunkSystem *Room::getChunkSystem() const {
	return p->chunk_system.get();
}

PluginManager *Room::getPluginManager() const {
	return p->plugin_manager.get();
}

void Room::freeRemovedSessions() {
	LockGuard lock(mtx_sessions);

	uniqdata<std::shared_ptr<Session>> to_remove;

	for(auto it = sessions.begin(); it != sessions.end();) {
		auto session = it->lock();
		if(!session) {
			it = sessions.erase(it);
		} else {
			if(session->hasStopped()) {
				to_remove.push_back(session);
			}
			it++;
		}
	}

	for(size_t i = 0; i < to_remove.size(); i++) {
		auto &session = to_remove[i];
		removeSession_nolock(session);
		server->removeSession(session->getConnection().get());
	}
}

void Room::log(const char *name, const char *format, ...) {
	char *msg = nullptr;

	va_list arglist;
	va_start(arglist, format);
	vasprintf(&msg, format, arglist);
	va_end(arglist);

	char room_name[64];
	snprintf(room_name, sizeof(room_name), "Room %s", getName().c_str());

	server->log(room_name, COLOR_BLUE "[%s]" COLOR_RESET " %s", name, msg);

	free(msg);
}

void Room::broadcast_nolock(const Packet &packet, Session *except) {
	// For every session
	for(auto &s : sessions) {
		auto session = s.lock();
		if(!session) continue;

		if(session.get() == except)
			continue;

		if(!session->isValid())
			continue;

		session->pushPacket(packet);
	}
}

void Room::broadcast(const Packet &packet, Session *except) {
	LockGuard lock(mtx_sessions);
	broadcast_nolock(packet, except);
}

void Room::forEverySessionExcept(Session *except, std::function<void(Session *)> callback) {
	LockGuard lock(mtx_sessions);

	// For every session
	for(auto &s : sessions) {
		auto session = s.lock();
		if(!session) continue;

		if(session.get() == except) continue;

		if(!session->isValid() || session->isStopping() || session->hasStopped())
			continue;

		callback(session.get());
	}
}

void Room::setPixels_nolock(GlobalPixel *pixels, u32 count) {
	Chunk *chunk_cache = nullptr;
	std::vector<ChunkPixel> *data_cache = nullptr;
	Int2 chunk_cache_pos = {INT32_MIN, INT32_MIN};

	std::map<Chunk *, std::vector<ChunkPixel>> chunk_data;

	for(u32 i = 0; i < count; i++) {
		auto &pixel = pixels[i];
		auto chunk_pos = ChunkSystem::globalPixelPosToChunkPos(pixel.pos);

		Chunk *chunk;
		std::vector<ChunkPixel> *data;
		if(chunk_pos == chunk_cache_pos) {
			chunk = chunk_cache;
			data = data_cache;
		} else {
			chunk = getChunkSystem()->getChunk(chunk_pos);
			if(!chunk) continue;
			data = &chunk_data[chunk];
			chunk_cache = chunk;
			data_cache = data;
		}

		ChunkPixel chunk_pixel;
		chunk_pixel.pos = ChunkSystem::globalPixelPosToLocalPixelPos(pixel.pos);
		chunk_pixel.r = pixel.r;
		chunk_pixel.g = pixel.g;
		chunk_pixel.b = pixel.b;
		data->push_back(chunk_pixel);
	}

	for(auto &data : chunk_data) {
		data.first->setPixels(data.second.data(), data.second.size());
	}
}