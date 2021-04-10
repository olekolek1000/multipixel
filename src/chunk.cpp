#include "chunk.hpp"
#include "chunk_system.hpp"
#include "session.hpp"
#include <cassert>

Chunk::Chunk(ChunkSystem *chunk_system, Int2 position)
		: chunk_system(chunk_system),
			position(position),
			chunk_size(chunk_system->getChunkSize()) {
}

Chunk::~Chunk() {
	//Remove linked sessions
	LockGuard lock(mtx_linked_sessions);

	//Remove linked chunks
	while(!linked_sessions.empty()) {
		linked_sessions.back()->unlinkChunk(this);
		linked_sessions.pop_back();
	}
}

void Chunk::allocateImage_nolock() {
	if(!image) {
		image.create(chunk_size * chunk_size * 3 /* RGB */);
		memset(image.data(), 255, image.size_bytes()); //White
	}
}

void Chunk::getPixel_nolock(UInt2 chunk_pixel_pos, u8 *r, u8 *g, u8 *b) {
	assert(chunk_pixel_pos.x < chunk_size);
	assert(chunk_pixel_pos.y < chunk_size);
	auto *rgb = image.data();
	size_t offset = chunk_pixel_pos.y * chunk_size * 3 + chunk_pixel_pos.x * 3;
	*r = rgb[offset + 0];
	*g = rgb[offset + 1];
	*b = rgb[offset + 2];
}

void Chunk::linkSession(Session *session) {
	LockGuard lock(mtx_linked_sessions);

	//Check if session pointer already exists
	for(auto &cell : linked_sessions) {
		if(cell == session)
			return;
	}

	//Add session pointer
	linked_sessions.push_back(session);
}

void Chunk::unlinkSession(Session *session) {
	LockGuard lock(mtx_linked_sessions);

	//Find pointer
	for(auto it = linked_sessions.begin(); it != linked_sessions.end();) {
		if(*it == session) {
			//Remove session pointer
			it = linked_sessions.erase(it);
			return;
		} else {
			it++;
		}
	}
}

void Chunk::setPixels(ChunkPixel *pixels, size_t count) {
	LockGuard lock(mtx_access);

	allocateImage_nolock();

	u8 r, g, b;
	for(size_t i = 0; i < count; i++) {
		auto &pixel = pixels[i];

		getPixel_nolock(pixel.pos, &r, &g, &b);
		if(pixel.r == r && pixel.g == g && pixel.b == b) {
			//Pixel not changed, skip
			continue;
		}

		//Update pixel
		auto *rgb = image.data();
		size_t offset = pixel.pos.y * chunk_size * 3 + pixel.pos.x * 3;
		rgb[offset + 0] = pixel.r;
		rgb[offset + 1] = pixel.g;
		rgb[offset + 2] = pixel.b;

		chunk_system->server->log("Updated chunk pixel at %u,%u", pixel.pos.x, pixel.pos.y);
	}
}

Int2 Chunk::getPosition() const {
	return position;
}
