#include "chunk.hpp"
#include "chunk_system.hpp"
#include "command.hpp"
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

void Chunk::sendChunkDataToSession_nolock(Session *session) {
	allocateImage_nolock();

	//Compress chunk data
	auto compressed_rgb = compressLZ4(image.data(), image.size_bytes());

	s32 chunk_x_BE = tobig32((s32)getPosition().x);
	s32 chunk_y_BE = tobig32((s32)getPosition().y);
	u32 raw_size_BE = tobig32((u32)image.size_bytes());

	Datasize data_chunk_x(&chunk_x_BE, sizeof(s32));
	Datasize data_chunk_y(&chunk_y_BE, sizeof(s32));
	Datasize data_raw_size(&raw_size_BE, sizeof(u32));
	Datasize data_compressed_data(compressed_rgb.data(), compressed_rgb.size_bytes());

	Datasize *datasizes[] = {
			&data_chunk_x,
			&data_chunk_y,
			&data_raw_size,
			&data_compressed_data,
			nullptr};

	session->pushPacket(preparePacket(ServerCmd::chunk_image, datasizes));
}

void Chunk::linkSession(Session *session) {
	LockGuard lock1(mtx_access);
	LockGuard lock2(mtx_linked_sessions);

	session->pushPacket(preparePacketChunkCreate(getPosition()));

	//Check if session pointer already exists
	for(auto &cell : linked_sessions) {
		if(cell == session)
			return;
	}

	//Add session pointer
	linked_sessions.push_back(session);

	sendChunkDataToSession_nolock(session);
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

	//Do nothing otherwise
}

void Chunk::setPixels(ChunkPixel *pixels, size_t count) {
	LockGuard lock(mtx_access);

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
}

Int2 Chunk::getPosition() const {
	return position;
}
