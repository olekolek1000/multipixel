#include "chunk_system.hpp"
#include "chunk.hpp"
#include "server.hpp"
#include "session.hpp"
#include "util/mutex.hpp"
#include "util/types.hpp"
#include <cassert>
#include <vector>

ChunkSystem::ChunkSystem(Server *server)
		: server(server) {

	server->dispatcher_session_remove.add(listener_session_remove, [this](Session *removing_session) {
		LockGuard lock(mtx_chunks);
		//For every chunk
		for(auto &i : chunks) {			//X
			for(auto &j : i.second) { //Y
				auto *chunk = j.second.get();
				chunk->unlinkSession(removing_session);
			}
		}
	});
}

ChunkSystem::~ChunkSystem() {
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

	struct ChunkCacheCell {
		Int2 chunk_pos;
		Chunk *chunk;
		std::vector<ChunkPixel> queued_pixels;
	};

	//Chunk "cache"
	//Visible and affected chunks by player
	std::vector<ChunkCacheCell> affected_chunks;

	auto fetchCell = [&](Int2 chunk_pos) -> ChunkCacheCell * {
		for(auto &cell : affected_chunks) {
			if(cell.chunk_pos == chunk_pos) {
				return &cell;
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
		if(fetchCell(chunk_pos) == nullptr) {
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
	s32 chunkX = pixel_pos.x / (s32)getChunkSize();
	s32 chunkY = pixel_pos.y / (s32)getChunkSize();

	if(pixel_pos.x < 0)
		chunkX--;

	if(pixel_pos.y < 0)
		chunkY--;

	return {chunkX, chunkY};
}

UInt2 ChunkSystem::globalPixelPosToLocalPixelPos(Int2 global_pixel_pos) {
	auto chunk_size = (s32)getChunkSize();

	s32 x = global_pixel_pos.x % chunk_size;
	s32 y = global_pixel_pos.y % chunk_size;

	if(global_pixel_pos.x < 0)
		x += chunk_size - 1;

	if(global_pixel_pos.y < 0)
		y += chunk_size - 1;

	//Can be removed later
	assert(x >= 0 && y >= 0 && x < chunk_size && y < chunk_size);

	return {(u32)x, (u32)y};
}

void ChunkSystem::announceChunkForSession(Session *session, Int2 chunk_pos) {
	LockGuard lock(mtx_chunks);
	auto *chunk = getChunk_nolock(chunk_pos);
	session->linkChunk(chunk);
}

void ChunkSystem::deannounceChunkForSession(Session *session, Int2 chunk_pos) {
	LockGuard lock(mtx_chunks);
	auto *chunk = getChunk_nolock(chunk_pos);
	session->unlinkChunk(chunk);
}