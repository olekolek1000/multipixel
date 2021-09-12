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
		uniqdata<u8> compressed_chunk_data;
		{
			//Load chunk pixels from database
			std::lock_guard lock(mtx_database);
			auto record = database.loadBytes(chunk_pos.x, chunk_pos.y);

			if(!record.data.empty())
				record.data.move_to(&compressed_chunk_data);
		}

		//Chunk not found, create new chunk
		auto &cell = horizontal[chunk_pos.y];
		cell.create(this, chunk_pos, &compressed_chunk_data);
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

void ChunkSystem::autosave() {
	std::lock_guard lock(mtx_access);

	std::vector<Chunk *> to_autosave;
	u32 total_chunk_count = 0;
	u32 saved_chunk_count = 0;

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

	if(saved_chunk_count)
		server->log("Autosaved %u chunks (%u chunks loaded)", saved_chunk_count, total_chunk_count);
}

void ChunkSystem::saveChunk_nolock(Chunk *chunk) {
	auto chunk_data = chunk->encodeChunkData();
	database.saveBytes(chunk->getPosition().x, chunk->getPosition().y, chunk_data.data(), chunk_data.size_bytes(), COMPRESSION_TYPE::LZ4);
	chunk->setModified(false);
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
	last_autosave_timestamp = getMillis();
	last_garbage_collect_timestamp = getMillis();

	while(running) {
		bool used = runner_tick();
		if(!used) {
			//Idle
			std::this_thread::sleep_for(std::chrono::milliseconds(10));
		}
	}

	autosave();
}

bool ChunkSystem::runner_tick() {
	bool used = false;

	auto millis = getMillis();

	if(last_autosave_timestamp + 30000 < millis) { //Autosave
		autosave();
		last_autosave_timestamp = millis;
	}

	if(last_garbage_collect_timestamp + 10000 < millis) {
		needs_garbage_collect = true;
		last_garbage_collect_timestamp = millis;
	}

	//Atomic operation:
	//if(needs_garbage_collect) { needs_garbage_collect = false; (...) }
	if(needs_garbage_collect.exchange(false)) {
		std::lock_guard lock(mtx_access);

		bool done = false;

		//Informational use only
		u32 saved_chunk_count = 0;
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
						//Save chunk data to database (only if modified)
						if(chunk->isModified()) {
							saved_chunk_count++;
							saveChunk_nolock(chunk);
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
			server->log("Saved %u chunks, %u total chunks loaded, %u removed (GC))", saved_chunk_count, loaded_chunk_count, removed_chunk_count);
	}

	return used;
}