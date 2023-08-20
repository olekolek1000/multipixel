#include "chunk_system.hpp"
#include "chunk.hpp"
#include "room.hpp"
#include "server.hpp"
#include "session.hpp"
#include "util/timestep.hpp"
#include "util/types.hpp"
#include <cassert>
#include <mutex>
#include <thread>
#include <vector>

static const char *LOG_CHUNK = "ChunkSystem";

ChunkSystem::ChunkSystem(Room *room)
		: room(room) {

	running = true;
	needs_garbage_collect = false;
	step_ticks.setRate(20);

	thr_runner = std::thread([this] {
		runner();
	});

	room->dispatcher_session_remove.add(listener_session_remove, [this](Session *removing_session) {
		LockGuard lock(mtx_access);
		// For every chunk
		for(auto &i : chunks) {			// X
			for(auto &j : i.second) { // Y
				auto *chunk = j.second.get();
				deannounceChunkForSession_nolock(removing_session, chunk->getPosition());
			}
		}
	});
}

ChunkSystem::~ChunkSystem() {
	running = false;
	if(thr_runner.joinable())
		thr_runner.join();
}

Chunk *ChunkSystem::getChunk(Int2 chunk_pos) {
	LockGuard lock(mtx_access);
	return getChunk_nolock(chunk_pos);
}

Chunk *ChunkSystem::getChunk_nolock(Int2 chunk_pos) {
	if(last_accessed_chunk_cache && last_accessed_chunk_cache->position == chunk_pos)
		return last_accessed_chunk_cache;

	auto &horizontal = chunks[chunk_pos.x];
	auto it = horizontal.find(chunk_pos.y);
	if(it == horizontal.end()) {
		SharedVector<u8> compressed_chunk_data;
		{
			// Load chunk pixels from database
			room->database.lock();
			auto record = room->database.chunkLoadData(chunk_pos);
			room->database.unlock();

			if(!record.data || !record.data->empty())
				compressed_chunk_data = record.data;
		}

		// Chunk not found, create new chunk
		auto &cell = horizontal[chunk_pos.y];
		cell.create(this, chunk_pos, compressed_chunk_data);
		last_accessed_chunk_cache = cell.get();
		return cell.get();
	} else {
		last_accessed_chunk_cache = it->second.get();
		return it->second.get();
	}
}

bool ChunkSystem::getPixel(Int2 global_pixel_pos, u8 *r, u8 *g, u8 *b) {
	LockGuard lock(mtx_access);

	auto chunk_pos = globalPixelPosToChunkPos(global_pixel_pos);

	auto local_pixel_pos = globalPixelPosToLocalPixelPos(global_pixel_pos);

	auto *chunk = getChunk_nolock(chunk_pos);
	chunk->lock();
	chunk->allocateImage_nolock();
	chunk->getPixel_nolock(local_pixel_pos, r, g, b);
	chunk->unlock();

	return true;
}

Int2 ChunkSystem::globalPixelPosToChunkPos(Int2 pixel_pos) {
	s32 chunkX = (pixel_pos.x + (pixel_pos.x < 0 ? 1 : 0)) / (s32)getChunkSize();
	s32 chunkY = (pixel_pos.y + (pixel_pos.y < 0 ? 1 : 0)) / (s32)getChunkSize();

	if(pixel_pos.x < 0)
		chunkX--;

	if(pixel_pos.y < 0)
		chunkY--;

	return {chunkX, chunkY};
}

int modulo(int x, int n) {
	return (x % n + n) % n;
}

UInt2 ChunkSystem::globalPixelPosToLocalPixelPos(Int2 global_pixel_pos) {
	auto chunk_size = (s32)getChunkSize();

	s32 x = modulo(global_pixel_pos.x, chunk_size);
	s32 y = modulo(global_pixel_pos.y, chunk_size);

	// Can be removed later
	assert(x >= 0 && y >= 0 && x < chunk_size && y < chunk_size);

	return {(u32)x, (u32)y};
}

void ChunkSystem::announceChunkForSession(Session *session, Int2 chunk_pos) {
	LockGuard lock(mtx_access);
	announceChunkForSession_nolock(session, chunk_pos);
}

void ChunkSystem::deannounceChunkForSession(Session *session, Int2 chunk_pos) {
	LockGuard lock(mtx_access);
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

void ChunkSystem::autosave() {
	auto start = getMillis();
	LockGuard lock(mtx_access);

	std::vector<Chunk *> to_autosave;
	u32 total_chunk_count = 0;
	u32 saved_chunk_count = 0;

	auto transaction = room->database.transactionBegin();

	for(auto &i : chunks) {
		for(auto &j : i.second) {
			auto *chunk = j.second.get();
			total_chunk_count++;

			if(chunk->isModified()) {
				saveChunk_nolock(chunk);
				saved_chunk_count++;
			}
		}
	}

	transaction->commit();

	if(saved_chunk_count) {
		u32 dur = getMillis() - start;
		room->log(LOG_CHUNK, "Autosaved %u chunks in %ums (%u chunks loaded)", saved_chunk_count, dur, total_chunk_count);
	}
}

void ChunkSystem::saveChunk_nolock(Chunk *chunk) {
	auto chunk_data = chunk->encodeChunkData(true);
	room->database.chunkSaveData(chunk->getPosition(), chunk_data->data(), chunk_data->size(), CompressionType::LZ4);
}

void ChunkSystem::removeChunk_nolock(Chunk *to_remove) {
	if(to_remove == last_accessed_chunk_cache)
		last_accessed_chunk_cache = nullptr;

	for(auto it = chunks.begin(); it != chunks.end(); it++) {
		for(auto jt = it->second.begin(); jt != it->second.end();) {
			if(jt->second.get() == to_remove) {
				it->second.erase(jt);

				// Remove empty map row
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
	last_autosave_timestamp = getMillis();
	last_garbage_collect_timestamp = getMillis();

	while(running) {
		bool used = runner_tick();
		if(!used) {
			// Idle
			std::this_thread::sleep_for(std::chrono::milliseconds(10));
		}
	}

	autosave();
}

bool ChunkSystem::runner_tick() {
	bool used = false;

	auto millis = getMillis();

	if(last_autosave_timestamp + room->settings.autosave_interval < millis) { // Autosave
		autosave();
		last_autosave_timestamp = millis;
	}

	if(last_garbage_collect_timestamp + 10000 < millis) {
		needs_garbage_collect = true;
		last_garbage_collect_timestamp = millis;
	}

	// Atomic operation:
	// if(needs_garbage_collect) { needs_garbage_collect = false; (...) }
	if(needs_garbage_collect.exchange(false)) {
		LockGuard lock(mtx_access);

		bool done = false;

		// Informational use only
		u32 saved_chunk_count = 0;
		u32 removed_chunk_count = 0;
		u32 loaded_chunk_count = 0;

		do {
			done = true;

			loaded_chunk_count = 0;

			// Iterate all loaded chunks as long as all chunks are deallocated
			for(auto &i : chunks) {
				loaded_chunk_count += i.second.size();
				for(auto &j : i.second) {
					auto *chunk = j.second.get();
					if(chunk->isLinkedSessionsEmpty()) {
						// Save chunk data to database (only if modified)
						if(chunk->isModified()) {
							saved_chunk_count++;
							room->database.lock();
							saveChunk_nolock(chunk);
							room->database.unlock();
						}
						removed_chunk_count++;
						removeChunk_nolock(chunk);
						done = false;
						goto breakloop;
					}
				}
			}

		breakloop:;

		} while(!done);

		if(saved_chunk_count || removed_chunk_count)
			room->log(LOG_CHUNK, "Saved %u chunks, %u total chunks loaded, %u removed (GC))", saved_chunk_count, loaded_chunk_count, removed_chunk_count);
	}

	while(step_ticks.onTick()) {
		used = true;

		if(ticks % 20 == 0) {
			LockGuard lock(mtx_access);
			for(auto &i : chunks) {
				for(auto &j : i.second) {
					j.second->flushQueuedPixels();
				}
			}
		}

		ticks++;
	}

	return used;
}