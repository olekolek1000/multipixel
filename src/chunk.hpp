#pragma once

#include "color.hpp"
#include "server.hpp"
#include "util/smartptr.hpp"
#include "util/types.hpp"
#include <atomic>
#include <memory>
#include <mutex>
#include <stack>
#include <vector>

struct ChunkSystem;
struct Session;

struct ChunkPixel {
	UInt2 pos;
	Color color;
};

struct Chunk {
	u32 getImageSizeBytes() const;

private:
	bool new_chunk = true; // Newly (and blank) chunk?
	ChunkSystem *chunk_system;
	Int2 position;

	/// @brief Dirty = modified chunk that should be saved
	std::atomic<bool> modified = false;

	std::vector<ChunkPixel> queued_pixels_to_send;

	Mutex mtx_access;

	SharedVector<u8> image;
	SharedVector<u8> compressed_image;

	bool send_chunk_data_instead_of_pixels = false;

	std::atomic<bool> linked_sessions_empty = true;
	std::vector<Session *> linked_sessions;

	void sendChunkDataToSession_nolock(Session *session);
	SharedVector<u8> encodeChunkData_nolock();
	void setModified_nolock(bool n);

public:
	Chunk(ChunkSystem *chunk_system, Int2 position, SharedVector<u8> compressed_chunk_data);
	~Chunk();

	friend struct ChunkSystem;

	void linkSession(Session *session);
	void unlinkSession(Session *session);
	bool isLinkedSessionsEmpty();

	void allocateImage_nolock();

	/// @param clear_modified Set to true if encoded chunk data will be used to save, raw RGB data will be freed
	SharedVector<u8> encodeChunkData(bool clear_modified);
	bool isModified();

	void setPixels(ChunkPixel *pixels, size_t count);
	void setPixels_nolock(ChunkPixel *pixels, size_t count, bool only_send = false);

	// Set pixel and send it later (delayed send)
	void setPixelQueued(ChunkPixel *pixel);
	void setPixelsQueued_nolock(ChunkPixel *pixels, u32 count);
	void setPixelQueued_nolock(ChunkPixel *pixel);

	void flushQueuedPixels();
	void flushQueuedPixels_nolock();

	Int2 getPosition() const;

	void getPixel_nolock(UInt2 chunk_pixel_pos, Color *color);

	void lock();
	void unlock();
};
