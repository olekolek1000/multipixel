#include "chunk.hpp"
#include "chunk_system.hpp"
#include "command.hpp"
#include "preview_system.hpp"
#include "room.hpp"
#include "server.hpp"
#include "session.hpp"
#include <cassert>

Chunk::Chunk(ChunkSystem *chunk_system, Int2 position, SharedVector<u8> compressed_chunk_data)
		: chunk_system(chunk_system),
			position(position) {
	LockGuard lock(mtx_access);
	this->compressed_image = compressed_chunk_data;

	new_chunk = true;
	if(compressed_chunk_data && !compressed_chunk_data->empty())
		new_chunk = false;
}

Chunk::~Chunk() {
	while(!linked_sessions.empty()) {
		fprintf(stderr, "Chunk linked sessions NOT empty");
		abort();
	}
}

u32 Chunk::getImageSizeBytes() const {
	return ChunkSystem::getChunkSize() * ChunkSystem::getChunkSize() * 3; /*RGB*/
}

void Chunk::allocateImage_nolock() {
	if(!image) {
		image = createSharedVector<u8>(getImageSizeBytes());
		new_chunk = false;

		if(compressed_image) {
			decompressLZ4(compressed_image->data(), compressed_image->size(), image->data(), image->size());
		} else {
			memset(image->data(), 255, image->size()); // White
		}
	}
}

void Chunk::getPixel_nolock(UInt2 chunk_pixel_pos, u8 *r, u8 *g, u8 *b) {
	assert(chunk_pixel_pos.x < ChunkSystem::getChunkSize());
	assert(chunk_pixel_pos.y < ChunkSystem::getChunkSize());
	auto *rgb = image->data();
	size_t offset = chunk_pixel_pos.y * ChunkSystem::getChunkSize() * 3 + chunk_pixel_pos.x * 3;
	*r = rgb[offset + 0];
	*g = rgb[offset + 1];
	*b = rgb[offset + 2];
}

static SharedVector<u8> compressed_empty_chunk;
static Mutex mtx_empty_chunk;

// Returns LZ4-compressed empty, white chunk. Generates once.
SharedVector<u8> getEmptyChunk(Chunk *chunk) {
	LockGuard lock(mtx_empty_chunk);
	if(!compressed_empty_chunk) {
		uniqdata<u8> stub_img(chunk->getImageSizeBytes());
		memset(stub_img.data(), 255, stub_img.size_bytes());
		compressed_empty_chunk = compressLZ4(stub_img.data(), stub_img.size_bytes());
	}
	return compressed_empty_chunk;
}

SharedVector<u8> Chunk::encodeChunkData_nolock() {
	SharedVector<u8> compressed;

	if(new_chunk) {
		// Return compressed empty chunk
		compressed = getEmptyChunk(this);
	} else {
		allocateImage_nolock();
		compressed = compressLZ4(image->data(), image->size());
	}

	this->compressed_image = compressed;
	return compressed;
}

SharedVector<u8> Chunk::encodeChunkData(bool clear_modified) {
	LockGuard lock(mtx_access);
	auto compressed = encodeChunkData_nolock();
	if(clear_modified) {
		setModified_nolock(false);
		image.reset();

		auto *preview_system = chunk_system->room->getPreviewSystem();
		preview_system->addToQueueFront({position.x / 2, position.y / 2});
	}
	return compressed;
}

void Chunk::sendChunkDataToSession_nolock(Session *session) {
	SharedVector<u8> compressed_data;

	if(compressed_image) {
		// Grab from cache
		compressed_data = compressed_image;
	} else {
		// Recompress chunk data
		compressed_data = encodeChunkData_nolock();
	}

	s32 chunk_x_BE = tobig32((s32)getPosition().x);
	s32 chunk_y_BE = tobig32((s32)getPosition().y);
	u32 raw_size_BE = tobig32((u32)getImageSizeBytes());

	Datasize data_chunk_x(&chunk_x_BE, sizeof(s32));
	Datasize data_chunk_y(&chunk_y_BE, sizeof(s32));
	Datasize data_raw_size(&raw_size_BE, sizeof(u32));
	Datasize data_compressed_data(compressed_data->data(), compressed_data->size());

	Datasize *datasizes[] = {
			&data_chunk_x,
			&data_chunk_y,
			&data_raw_size,
			&data_compressed_data,
			nullptr};

	session->pushPacket(preparePacket(ServerCmd::chunk_image, datasizes));
}

void Chunk::linkSession(Session *session) {
	LockGuard lock(mtx_access);

	// Check if session pointer already exists
	for(auto &cell : linked_sessions) {
		if(cell == session)
			return;
	}

	linked_sessions_empty = false;

	// Add session pointer
	linked_sessions.push_back(session);

	sendChunkDataToSession_nolock(session);
}

void Chunk::unlinkSession(Session *session) {
	LockGuard lock(mtx_access);

	// Find pointer
	for(auto it = linked_sessions.begin(); it != linked_sessions.end();) {
		if(*it == session) {
			// Remove session pointer
			it = linked_sessions.erase(it);
			break;
		} else {
			it++;
		}
	}

	bool is_empty = linked_sessions.empty();
	linked_sessions_empty = is_empty;

	if(is_empty)
		chunk_system->markGarbageCollect();
}

bool Chunk::isLinkedSessionsEmpty() {
	return linked_sessions_empty;
}

void Chunk::lock() {
	mtx_access.lock();
}

void Chunk::unlock() {
	mtx_access.unlock();
}

void Chunk::setPixelQueued(ChunkPixel *pixel) {
	LockGuard lock(mtx_access);
	allocateImage_nolock();

	auto *rgb = image->data();
	size_t offset = pixel->pos.y * ChunkSystem::getChunkSize() * 3 + pixel->pos.x * 3;
	rgb[offset + 0] = pixel->r;
	rgb[offset + 1] = pixel->g;
	rgb[offset + 2] = pixel->b;

	queued_pixels_to_send.push_back(*pixel);
	setModified_nolock(true);
}

void Chunk::flushQueuedPixels() {
	LockGuard lock(mtx_access);
	flushSendDelay_nolock();
}

void Chunk::flushSendDelay_nolock() {
	if(queued_pixels_to_send.empty()) return;
	setPixels_nolock(queued_pixels_to_send.data(), queued_pixels_to_send.size(), true);
	queued_pixels_to_send.clear();
}

void Chunk::setPixels(ChunkPixel *pixels, size_t count) {
	LockGuard lock(mtx_access);
	flushSendDelay_nolock();
	setPixels_nolock(pixels, count);
}

void Chunk::setPixels_nolock(ChunkPixel *pixels, size_t count, bool only_send) {
	allocateImage_nolock();

	// Prepare pixel_pack packet
	Buffer buf_pixels;
	u32 pixel_count = 0;

	u8 r, g, b;
	for(size_t i = 0; i < count; i++) {
		auto &pixel = pixels[i];

		if(!only_send) {
			getPixel_nolock(pixel.pos, &r, &g, &b);
			if(pixel.r == r && pixel.g == g && pixel.b == b) {
				// Pixel not changed, skip
				continue;
			}

			// Update pixel
			auto *rgb = image->data();
			size_t offset = pixel.pos.y * ChunkSystem::getChunkSize() * 3 + pixel.pos.x * 3;
			rgb[offset + 0] = pixel.r;
			rgb[offset + 1] = pixel.g;
			rgb[offset + 2] = pixel.b;
		}

		// Prepare pixel data
		u8 x = pixel.pos.x;
		u8 y = pixel.pos.y;

		buf_pixels.write(&x, sizeof(u8));
		buf_pixels.write(&y, sizeof(u8));
		buf_pixels.write(&pixel.r, sizeof(u8));
		buf_pixels.write(&pixel.g, sizeof(u8));
		buf_pixels.write(&pixel.b, sizeof(u8));
		pixel_count++;
	}

	if(pixel_count == 0)
		return; // Nothing modified

	auto compressed = compressLZ4(buf_pixels.data(), buf_pixels.size());

	s32 chunk_x_BE = tobig32((s32)getPosition().x);
	s32 chunk_y_BE = tobig32((s32)getPosition().y);
	u32 pixel_count_BE = tobig32((u32)pixel_count);
	u32 raw_size_BE = tobig32((u32)buf_pixels.size());

	Datasize data_chunk_x(&chunk_x_BE, sizeof(s32));
	Datasize data_chunk_y(&chunk_y_BE, sizeof(s32));
	Datasize data_pixel_count(&pixel_count_BE, sizeof(u32));
	Datasize data_raw_size(&raw_size_BE, sizeof(u32));
	Datasize data_compressed_data(compressed->data(), compressed->size());

	Datasize *datasizes[] = {
			&data_chunk_x,
			&data_chunk_y,
			&data_pixel_count,
			&data_raw_size,
			&data_compressed_data,
			nullptr};

	auto packet = preparePacket(ServerCmd::chunk_pixel_pack, datasizes);

	for(auto &session : linked_sessions) {
		session->pushPacket(packet);
	}

	setModified_nolock(true);
}

Int2 Chunk::getPosition() const {
	return position;
}

bool Chunk::isModified() {
	return modified;
}

void Chunk::setModified_nolock(bool n) {
	modified = n;
	if(modified) {
		// Compressed image data is now invalid
		compressed_image.reset();
	}
}
