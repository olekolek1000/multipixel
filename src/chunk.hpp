#pragma once

#include "server.hpp"
#include "util/smartptr.hpp"
#include "util/types.hpp"
#include <atomic>
#include <memory>
#include <mutex>
#include <vector>

struct ChunkSystem;
struct Session;

struct ChunkPixel {
	UInt2 pos;
	u8 r, g, b;
};

struct Chunk {
private:
	ChunkSystem *chunk_system;
	Int2 position;
	u32 chunk_size;

	/// @brief Dirty = modified chunk that should be saved
	std::atomic<bool> modified = false;

	std::mutex mtx_access;

	uniqdata<u8> image;
	SharedVector<u8> compressed_image;

	std::atomic<bool> linked_sessions_empty = true;
	std::vector<Session *> linked_sessions;

	void allocateImage_nolock();
	void sendChunkDataToSession_nolock(Session *session);
	SharedVector<u8> encodeChunkData_nolock();
	void decodeChunkData_nolock(const SharedVector<u8> &compressed_chunk_data);
	void getPixel_nolock(UInt2 chunk_pixel_pos, u8 *r, u8 *g, u8 *b);
	void setModified_nolock(bool n);

public:
	Chunk(ChunkSystem *chunk_system, Int2 position, SharedVector<u8> compressed_chunk_data);
	~Chunk();

	friend struct ChunkSystem;

	void linkSession(Session *session);
	void unlinkSession(Session *session);
	bool isLinkedSessionsEmpty();

	/// @param clear_modified Set to true if encoded chunk data will be used to save
	SharedVector<u8> encodeChunkData(bool clear_modified);
	bool isModified();
	void setModified(bool n);

	void setPixels(ChunkPixel *pixels, size_t count);
	Int2 getPosition() const;
};
