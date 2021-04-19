#include "chunk.hpp"
#include "chunk_system.hpp"
#include "command.hpp"
#include "session.hpp"
#include <cassert>

Chunk::Chunk(ChunkSystem *chunk_system, Int2 position, uniqdata<u8> *compressed_chunk_data)
		: chunk_system(chunk_system),
			position(position),
			chunk_size(chunk_system->getChunkSize()) {
	if(!compressed_chunk_data->empty()) {
		std::lock_guard lock(mtx_access);
		decodeChunkData_nolock(compressed_chunk_data->data(), compressed_chunk_data->size());
	}
}

Chunk::~Chunk() {
	while(!linked_sessions.empty()) {
		fprintf(stderr, "Chunk linked sessions NOT empty");
		abort();
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

uniqdata<u8> Chunk::encodeChunkData_nolock() {
	allocateImage_nolock();
	return compressLZ4(image.data(), image.size_bytes());
}

uniqdata<u8> Chunk::encodeChunkData() {
	std::lock_guard lock(mtx_access);
	return encodeChunkData_nolock();
}

void Chunk::decodeChunkData_nolock(const void *data, size_t size) {
	allocateImage_nolock();
	decompressLZ4(data, size, image.data(), image.size_bytes());
}

void Chunk::sendChunkDataToSession_nolock(Session *session) {
	allocateImage_nolock();

	//Compress chunk data
	auto compressed_data = encodeChunkData_nolock();

	s32 chunk_x_BE = tobig32((s32)getPosition().x);
	s32 chunk_y_BE = tobig32((s32)getPosition().y);
	u32 raw_size_BE = tobig32((u32)image.size_bytes());

	Datasize data_chunk_x(&chunk_x_BE, sizeof(s32));
	Datasize data_chunk_y(&chunk_y_BE, sizeof(s32));
	Datasize data_raw_size(&raw_size_BE, sizeof(u32));
	Datasize data_compressed_data(compressed_data.data(), compressed_data.size_bytes());

	Datasize *datasizes[] = {
			&data_chunk_x,
			&data_chunk_y,
			&data_raw_size,
			&data_compressed_data,
			nullptr};

	session->pushPacket(preparePacket(ServerCmd::chunk_image, datasizes));
}

void Chunk::linkSession(Session *session) {
	std::lock_guard lock(mtx_access);

	//Check if session pointer already exists
	for(auto &cell : linked_sessions) {
		if(cell == session)
			return;
	}

	linked_sessions_empty = false;

	//Add session pointer
	linked_sessions.push_back(session);

	sendChunkDataToSession_nolock(session);
}

void Chunk::unlinkSession(Session *session) {
	std::lock_guard lock(mtx_access);

	//Find pointer
	for(auto it = linked_sessions.begin(); it != linked_sessions.end();) {
		if(*it == session) {
			//Remove session pointer
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

void Chunk::setPixels(ChunkPixel *pixels, size_t count) {
	std::lock_guard lock(mtx_access);

	allocateImage_nolock();

	//Prepare pixel_pack packet
	Buffer buf_pixels;
	u32 pixel_count = 0;

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

		//Prepare pixel data
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
		return;

	auto compressed = compressLZ4(buf_pixels.data(), buf_pixels.size());

	s32 chunk_x_BE = tobig32((s32)getPosition().x);
	s32 chunk_y_BE = tobig32((s32)getPosition().y);
	u32 pixel_count_BE = tobig32((u32)pixel_count);
	u32 raw_size_BE = tobig32((u32)buf_pixels.size());

	Datasize data_chunk_x(&chunk_x_BE, sizeof(s32));
	Datasize data_chunk_y(&chunk_y_BE, sizeof(s32));
	Datasize data_pixel_count(&pixel_count_BE, sizeof(u32));
	Datasize data_raw_size(&raw_size_BE, sizeof(u32));
	Datasize data_compressed_data(compressed.data(), compressed.size_bytes());

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

	modified = true;
}

Int2 Chunk::getPosition() const {
	return position;
}

bool Chunk::isModified() {
	return modified;
}