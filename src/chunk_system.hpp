#pragma once

#include "database.hpp"
#include "util/listener.hpp"
#include "util/smartptr.hpp"
#include "util/types.hpp"
#include <atomic>
#include <map>
#include <mutex>
#include <thread>

struct Server;
struct Session;
struct Chunk;

struct GlobalPixel {
	Int2 pos;
	u8 r, g, b;
};

struct ChunkSystem {
	Server *server;

private:
	std::mutex mtx_database;
	DatabaseConnector database;

	std::mutex mtx_access;
	std::map<s32, std::map<s32, uniqptr<Chunk>>> chunks;

	std::atomic<bool> running;
	std::thread thr_runner;

	std::atomic<bool> needs_garbage_collect;

	Listener<void(Session *)> listener_session_remove;

public:
	ChunkSystem(Server *server);
	~ChunkSystem();

	static u32 getChunkSize() {
		return 256;
	}

	void setPixels(Session *session, GlobalPixel *pixels, size_t count);

	///@returns chunk coordinates from global pixel position
	Int2 globalPixelPosToChunkPos(Int2 global_pixel_pos);

	///@returns local chunk pixel position (e.g. 0-255)
	UInt2 globalPixelPosToLocalPixelPos(Int2 global_pixel_pos);

	void announceChunkForSession(Session *session, Int2 chunk_pos);
	void deannounceChunkForSession(Session *session, Int2 chunk_pos);

	void markGarbageCollect();

private:
	//Never returns null
	Chunk *getChunk_nolock(Int2 chunk_pos);

	//Save chunk to database and free it
	void removeChunk_nolock(Chunk *to_remove);

	void runner();
	bool runner_tick();

	void announceChunkForSession_nolock(Session *session, Int2 chunk_pos);
	void deannounceChunkForSession_nolock(Session *session, Int2 chunk_pos);
};