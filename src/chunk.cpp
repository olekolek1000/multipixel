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

void Chunk::getPixel_nolock(UInt2 chunk_pixel_pos, Color *color) {
	assert(chunk_pixel_pos.x < ChunkSystem::getChunkSize());
	assert(chunk_pixel_pos.y < ChunkSystem::getChunkSize());
	auto *rgb = image->data();
	size_t offset = chunk_pixel_pos.y * ChunkSystem::getChunkSize() * 3 + chunk_pixel_pos.x * 3;
	color->r = rgb[offset + 0];
	color->g = rgb[offset + 1];
	color->b = rgb[offset + 2];
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

		Int2 upper_pos;
		upper_pos.x = position.x >= 0 ? position.x / 2 : (position.x - 1) / 2;
		upper_pos.y = position.y >= 0 ? position.y / 2 : (position.y - 1) / 2;
		preview_system->addToQueueFront(upper_pos);
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

void Chunk::setPixelsQueued_nolock(ChunkPixel *pixels, u32 count) {
	allocateImage_nolock();
	auto *rgb = image->data();

	if(!send_chunk_data_instead_of_pixels) {
		queued_pixels_to_send.reserve(queued_pixels_to_send.size() + count);
	}

	for(u32 i = 0; i < count; i++) {
		auto &pixel = pixels[i];
		size_t offset = pixel.pos.y * ChunkSystem::getChunkSize() * 3 + pixel.pos.x * 3;
		rgb[offset + 0] = pixel.color.r;
		rgb[offset + 1] = pixel.color.g;
		rgb[offset + 2] = pixel.color.b;

		if(!send_chunk_data_instead_of_pixels) {
			queued_pixels_to_send.push_back(pixel);
			if(queued_pixels_to_send.size() > 5000) {
				queued_pixels_to_send = {};
				send_chunk_data_instead_of_pixels = true;
			}
		}
	}
	setModified_nolock(true);
}

void Chunk::setPixelQueued_nolock(ChunkPixel *pixel) {
	setPixelsQueued_nolock(pixel, 1);
}

void Chunk::setPixelQueued(ChunkPixel *pixel) {
	LockGuard lock(mtx_access);
	setPixelQueued_nolock(pixel);
}

void Chunk::flushQueuedPixels() {
	LockGuard lock(mtx_access);
	flushQueuedPixels_nolock();
}

void Chunk::flushQueuedPixels_nolock() {
	if(send_chunk_data_instead_of_pixels) {
		for(auto &session : linked_sessions) {
			sendChunkDataToSession_nolock(session);
		}
		send_chunk_data_instead_of_pixels = false;
	} else {
		if(queued_pixels_to_send.empty()) return;
		setPixels_nolock(queued_pixels_to_send.data(), queued_pixels_to_send.size(), true);
		queued_pixels_to_send.clear();
	}
}

void Chunk::setPixels(ChunkPixel *pixels, size_t count) {
	LockGuard lock(mtx_access);
	flushQueuedPixels_nolock();
	setPixels_nolock(pixels, count);
}

void Chunk::setPixels_nolock(ChunkPixel *pixels, size_t count, bool only_send) {
	allocateImage_nolock();

	// Prepare pixel_pack packet
	Buffer buf_pixels;
	u32 pixel_count = 0;

	Color color;
	for(size_t i = 0; i < count; i++) {
		auto &pixel = pixels[i];

		if(!only_send) {
			getPixel_nolock(pixel.pos, &color);
			if(pixel.color == color) {
				// Pixel not changed, skip
				continue;
			}

			// Update pixel
			auto *rgb = image->data();
			size_t offset = pixel.pos.y * ChunkSystem::getChunkSize() * 3 + pixel.pos.x * 3;
			rgb[offset + 0] = pixel.color.r;
			rgb[offset + 1] = pixel.color.g;
			rgb[offset + 2] = pixel.color.b;
		}

		// Prepare pixel data
		u8 x = pixel.pos.x;
		u8 y = pixel.pos.y;

		buf_pixels.write(&x, sizeof(u8));
		buf_pixels.write(&y, sizeof(u8));
		buf_pixels.write(&pixel.color.r, sizeof(u8));
		buf_pixels.write(&pixel.color.g, sizeof(u8));
		buf_pixels.write(&pixel.color.b, sizeof(u8));
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
