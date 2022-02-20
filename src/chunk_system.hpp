#pragma once

#include "database.hpp"
#include "util/listener.hpp"
#include "util/mutex.hpp"
#include "util/smartptr.hpp"
#include "util/timestep.hpp"
#include "util/types.hpp"
#include <atomic>
#include <map>
#include <mutex>
#include <thread>

struct Room;
struct Session;
struct Chunk;

struct ChunkSystem {
	Room *room;

private:
	Mutex mtx_access;
	std::map<s32, std::map<s32, uniqptr<Chunk>>> chunks;

	Chunk *last_accessed_chunk_cache = nullptr;

	std::atomic<bool> running;
	std::thread thr_runner;

	u64 last_autosave_timestamp;
	u64 last_garbage_collect_timestamp;

	std::atomic<bool> needs_garbage_collect;

	Listener<void(Session *)> listener_session_remove;

	Timestep step_ticks;
	u32 ticks = 0;

public:
	ChunkSystem(Room *room);
	~ChunkSystem();

	static u32 getChunkSize() {
		return 256;
	}

	bool getPixel(Int2 global_pixel_pos, u8 *r, u8 *g, u8 *b);

	///@returns chunk coordinates from global pixel position
	static Int2 globalPixelPosToChunkPos(Int2 global_pixel_pos);

	///@returns local chunk pixel position (e.g. 0-255)
	static UInt2 globalPixelPosToLocalPixelPos(Int2 global_pixel_pos);

	void announceChunkForSession(Session *session, Int2 chunk_pos);
	void deannounceChunkForSession(Session *session, Int2 chunk_pos);

	void markGarbageCollect();

	Chunk *getChunk(Int2 chunk_pos);

private:
	// Never returns null
	Chunk *getChunk_nolock(Int2 chunk_pos);

	// Save chunk to database and free it
	void removeChunk_nolock(Chunk *to_remove);

	void runner();
	bool runner_tick();

	void announceChunkForSession_nolock(Session *session, Int2 chunk_pos);
	void deannounceChunkForSession_nolock(Session *session, Int2 chunk_pos);

	void autosave();
	void saveChunk_nolock(Chunk *chunk);
};