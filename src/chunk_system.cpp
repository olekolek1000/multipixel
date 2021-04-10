#include "chunk_system.hpp"
#include "chunk.hpp"
#include "server.hpp"
#include "session.hpp"
#include "util/mutex.hpp"
#include "util/types.hpp"
#include <cassert>
#include <vector>

ChunkSystem::ChunkSystem(Server *server)
		: server(server),
			ended(false) {

	thr_runner = std::thread(&ChunkSystem::runner, this);
}

ChunkSystem::~ChunkSystem() {
	ended = true;
	server->log("Joining ChunkSystem thread");
	if(thr_runner.joinable())
		thr_runner.join();
}

void ChunkSystem::runner() {
	while(!ended) {
		if(runner_tick()) {
			continue;
		} else {
			//Idle
			std::this_thread::sleep_for(std::chrono::milliseconds(10));
		}
	}
}

bool ChunkSystem::runner_tick() {
	bool used = false;

	return used;
}

Chunk *ChunkSystem::getChunk_nolock(Int2 chunk_pos) {
	auto &horizontal = chunks[chunk_pos.x];
	auto it = horizontal.find(chunk_pos.y);
	if(it == horizontal.end()) {
		//Chunk not found, create new
		auto &cell = horizontal[chunk_pos.y];
		cell.create(this, chunk_pos);
		return cell.get();
	} else {
		return it->second.get();
	}
}

void ChunkSystem::setPixels(Session *session, GlobalPixel *pixels, size_t count) {
	LockGuard lock(mtx_chunks);

	//Chunk "cache"
	//Visible and affected chunks by player

	struct ChunkCacheCell {
		Int2 chunk_pos;
		Chunk *chunk;
		std::vector<ChunkPixel> queued_pixels;
	};

	std::vector<ChunkCacheCell> affected_chunks;

	auto fetchCell = [&](Int2 chunk_pos) -> ChunkCacheCell * {
		for(auto &cell : affected_chunks) {
			if(cell.chunk_pos == chunk_pos) {
				return &cell;
			}
		}

		return nullptr;
	};

	auto fetchChunk = [&](Int2 chunk_pos) -> Chunk * {
		for(auto &cell : affected_chunks) {
			if(cell.chunk_pos == chunk_pos) {
				return cell.chunk;
			}
		}

		return nullptr;
	};

	auto cacheNewChunk = [&](Int2 chunk_pos) -> Chunk * {
		if(!session->isChunkLinked(chunk_pos))
			return nullptr; //Out of client bounds. Prevent hacked clients from drawing outside client's loaded chunks.

		auto *chunk = getChunk_nolock(chunk_pos);
		affected_chunks.push_back({Int2(chunk_pos), chunk});
		return chunk;
	};

	//Generate affected chunks list
	for(size_t i = 0; i < count; i++) {
		auto &pixel = pixels[i];
		auto chunk_pos = globalPixelPosToChunkPos(pixel.pos);
		if(fetchChunk(chunk_pos) == nullptr) {
			cacheNewChunk(chunk_pos);
		}
	}

	//Send pixels to chunks
	for(size_t i = 0; i < count; i++) {
		auto &pixel = pixels[i];
		auto chunk_pos = globalPixelPosToChunkPos(pixel.pos);
		auto *cell = fetchCell(chunk_pos);
		if(!cell)
			continue; //Skip pixel

		auto &queued_pixel = cell->queued_pixels.emplace_back();
		queued_pixel.pos = globalPixelPosToLocalPixelPos(pixel.pos);
		queued_pixel.r = pixel.r;
		queued_pixel.g = pixel.g;
		queued_pixel.b = pixel.b;
	}

	for(auto &cell : affected_chunks) {
		if(cell.queued_pixels.empty())
			continue;

		cell.chunk->setPixels(cell.queued_pixels.data(), cell.queued_pixels.size());
	}
}

Int2 ChunkSystem::globalPixelPosToChunkPos(Int2 pixel_pos) {
	s32 chunkX = pixel_pos.x / getChunkSize();
	s32 chunkY = pixel_pos.y / getChunkSize();

	if(pixel_pos.x < 0)
		chunkX--;

	if(pixel_pos.y < 0)
		chunkY--;

	return {chunkX, chunkY};
}

UInt2 ChunkSystem::globalPixelPosToLocalPixelPos(Int2 global_pixel_pos) {
	auto chunk_size = getChunkSize();

	s32 x = global_pixel_pos.x % chunk_size;
	s32 y = global_pixel_pos.y % chunk_size;

	if(global_pixel_pos.x < 0)
		x += chunk_size;

	if(global_pixel_pos.y < 0)
		y += chunk_size;

	//Can be removed later
	assert(x >= 0 && y >= 0 && x < chunk_size && y < chunk_size);

	return {(u32)x, (u32)y};
}

void ChunkSystem::announceChunkForSession(Session *session, Int2 chunk_pos) {
	LockGuard lock(mtx_chunks);

	auto *chunk = getChunk_nolock(chunk_pos);
	chunk->linkSession(session);
	session->linkChunk(chunk);
}

void ChunkSystem::deannounceChunkForSession(Session *session, Int2 chunk_pos) {
	LockGuard lock(mtx_chunks);

	auto *chunk = getChunk_nolock(chunk_pos);
	chunk->unlinkSession(session);
	session->unlinkChunk(chunk);
}