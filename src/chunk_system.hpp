#pragma once

#include "util/mutex.hpp"
#include "util/smartptr.hpp"
#include "util/types.hpp"
#include <map>
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
	bool ended;

	Mutex mtx_chunks;
	std::map<s32, std::map<s32, uniqptr<Chunk>>> chunks;

	std::thread thr_runner;

	void runner();
	bool runner_tick();

	//Never returns null
	Chunk *getChunk_nolock(Int2 chunk_pos);

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
};