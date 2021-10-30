#pragma once

#include "util/buffer.hpp"
#include "util/smartptr.hpp"
#include "util/types.hpp"
#include <memory>

#define NO_SANITIZER __attribute__((no_sanitize("undefined")))
#define PACKED			 __attribute__((packed))

enum struct ToolType {
	brush = 0,
	floodfill = 1
};

enum struct ClientCmd : u16 {
	message = 1,	//utf-8 text
	announce = 2, //utf-8 username
	ping = 4,
	cursor_pos = 100, //s32 x, s32 y
	cursor_down = 101,
	cursor_up = 102,
	boundary = 103,
	chunks_received = 104,
	tool_size = 200,	//u8 size
	tool_color = 201, //u8 red, u8 green, u8 blue
	tool_type = 202,	//u8 type
};

enum struct ServerCmd : u16 {
	message = 1,						//utf-8 text
	your_id = 2,						//u16 id
	kick = 3,								//utf-8 reason
	chunk_image = 100,			//complex data
	chunk_pixel_pack = 101, //complex data
	chunk_create = 110,			//s32 chunkX, s32 chunkY
	chunk_remove = 111,			//s32 chunkX, s32 chunkY
	user_create = 200,			//u16 id, utf-8 nickname
	user_remove = 201,			//u16 id
	user_cursor_pos = 202,	//u16 id, s32 x, s32 y
};

u16 frombig16(u16 in);
s16 frombig16(s16 in);
u32 frombig32(u32 in);
s32 frombig32(s32 in);
u32 frombig64(u64 in);
s32 frombig64(s64 in);
u16 tobig16(u16 in);
s16 tobig16(s16 in);
u32 tobig32(u32 in);
s32 tobig32(s32 in);
u64 tobig64(u64 in);
s64 tobig64(s64 in);

typedef std::shared_ptr<uniqdata<u8>> Packet;

struct Session;

struct Datasize {
	const void *data;
	u32 size;
	Datasize(const void *data, u32 size)
			: data(data), size(size) {}
};

Packet preparePacket(ServerCmd cmd, Datasize **datas);
Packet preparePacket(ServerCmd cmd, const void *data, u32 size);
Packet preparePacketUserCursorPos(u16 session_id, s32 x, s32 y);
Packet preparePacketUserCreate(Session *session);
Packet preparePacketUserRemove(Session *session);
Packet preparePacketChunkCreate(Int2 chunk_pos);
Packet preparePacketChunkRemove(Int2 chunk_pos);
Packet preparePacketMessage(const char *message);

template <typename T>
using SharedVector = std::shared_ptr<std::vector<T>>;

template <typename T>
inline SharedVector<T> createSharedVector(size_t count) {
	return std::make_shared<std::vector<T>>(count);
}

SharedVector<u8> compressLZ4(const void *data, u32 raw_size) NO_SANITIZER;

///@returns <= 0 on failure
int decompressLZ4(const void *compressed_data, u32 compressed_size, void *raw_data, u32 raw_size) NO_SANITIZER;