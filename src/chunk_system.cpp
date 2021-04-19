#include "chunk_system.hpp"
#include "chunk.hpp"
#include "server.hpp"
#include "session.hpp"
#include "util/types.hpp"
#include <cassert>
#include <mutex>
#include <thread>
#include <vector>

ChunkSystem::ChunkSystem(Server *server)
		: server(server) {

	running = true;
	needs_garbage_collect = false;
	thr_runner = std::thread([this] {
		runner();
	});

	server->dispatcher_session_remove.add(listener_session_remove, [this](Session *removing_session) {
		std::lock_guard lock(mtx_access);
		//For every chunk
		for(auto &i : chunks) {			//X
			for(auto &j : i.second) { //Y
				auto *chunk = j.second.get();
				deannounceChunkForSession_nolock(removing_session, chunk->getPosition());
			}
		}
	});
}

ChunkSystem::~ChunkSystem() {
	running = false;
	server->log("Joining chunk system runner thread");
	if(thr_runner.joinable())
		thr_runner.join();
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
	std::lock_guard lock(mtx_access);

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
	std::lock_guard lock(mtx_access);
	announceChunkForSession_nolock(session, chunk_pos);
}

void ChunkSystem::deannounceChunkForSession(Session *session, Int2 chunk_pos) {
	std::lock_guard lock(mtx_access);
	deannounceChunkForSession_nolock(session, chunk_pos);
}

void ChunkSystem::announceChunkForSession_nolock(Session *session, Int2 chunk_pos) {
	auto *chunk = getChunk_nolock(chunk_pos);
	session->linkChunk(chunk);
	chunk->linkSession(session);
}

void ChunkSystem::deannounceChunkForSession_nolock(Session *session, Int2 chunk_pos) {
	auto *chunk = getChunk_nolock(chunk_pos);
	session->unlinkChunk(chunk);
	chunk->unlinkSession(session);
}

void ChunkSystem::removeChunk_nolock(Chunk *to_remove) {
	for(auto it = chunks.begin(); it != chunks.end(); it++) {
		for(auto jt = it->second.begin(); jt != it->second.end();) {
			if(jt->second.get() == to_remove) {
				it->second.erase(jt);

				//Remove empty map row
				if(it->second.empty())
					it = chunks.erase(it);

				return;
			} else {
				jt++;
			}
		}
	}
}

void ChunkSystem::markGarbageCollect() {
	needs_garbage_collect = true;
}

void ChunkSystem::runner() {
	while(running) {
		bool used = runner_tick();
		if(!used) {
			//Idle
			std::this_thread::sleep_for(std::chrono::milliseconds(10));
		}
	}
}

bool ChunkSystem::runner_tick() {

	bool used = false;

	//Atomic operation:
	//if(needs_garbage_collect) { needs_garbage_collect = false; (...) }
	if(needs_garbage_collect.exchange(false)) {
		std::lock_guard lock(mtx_access);

		bool done = false;

		//Informational use only
		u32 removed_chunk_count = 0;
		u32 loaded_chunk_count = 0;

		do {
			done = true;

			loaded_chunk_count = 0;

			//Iterate all loaded chunks as long as all chunks are deallocated
			for(auto &i : chunks) {
				loaded_chunk_count += i.second.size();
				for(auto &j : i.second) {
					auto *chunk = j.second.get();
					if(chunk->isLinkedSessionsEmpty()) {
						removeChunk_nolock(chunk);
						done = false;
						removed_chunk_count++;
						goto breakloop;
					}
				}
			}

		breakloop:;

		} while(!done);

		server->log("Garbage collected %u chunks (%u chunks loaded)", removed_chunk_count, loaded_chunk_count);
	}

	return used;
}