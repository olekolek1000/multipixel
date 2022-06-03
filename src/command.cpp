#include "command.hpp"
#include "lz4.h"
#include "lz4hc.h"
#include "session.hpp"
#include "util/buffer.hpp"
#include "util/byteswap.hpp"
#include <cassert>

static_assert(sizeof(ClientCmd) == 2);
static_assert(sizeof(ServerCmd) == 2);

u16 frombig16(u16 in) {
	return bswap_16(in);
}

s16 frombig16(s16 in) {
	return bswap_16(in);
}

u32 frombig32(u32 in) {
	return bswap_32(in);
}

s32 frombig32(s32 in) {
	return bswap_32(in);
}

float frombig32(float in) {
	float ret;
	auto *a = (u8 *)&in;
	auto *b = (u8 *)&ret;
	b[0] = a[3];
	b[1] = a[2];
	b[2] = a[1];
	b[3] = a[0];
	return ret;
}

u32 frombig64(u64 in) {
	return bswap_64(in);
}

s32 frombig64(s64 in) {
	return bswap_64(in);
}

u16 tobig16(u16 in) {
	return bswap_16(in);
}

s16 tobig16(s16 in) {
	return bswap_16(in);
}

u32 tobig32(u32 in) {
	return bswap_32(in);
}

s32 tobig32(s32 in) {
	return bswap_32(in);
}

float tobig32(float in) {
	float ret;
	auto *a = (u8 *)&in;
	auto *b = (u8 *)&ret;
	b[0] = a[3];
	b[1] = a[2];
	b[2] = a[1];
	b[3] = a[0];
	return ret;
}

u64 tobig64(u64 in) {
	return bswap_64(in);
}

s64 tobig64(s64 in) {
	return bswap_64(in);
}

static Packet allocPacket() {
	return std::make_shared<uniqdata<u8>>();
}

Packet preparePacket(ServerCmd cmd, Datasize **datas) {
	auto packet = allocPacket();

	u32 total_size = 0;

	// Count size
	u32 index = 0;
	while(true) {
		auto *datasize = datas[index];
		if(datasize) {
			total_size += datasize->size;
			index++;
		} else {
			break;
		}
	}

	packet->resize(sizeof(ServerCmd) + total_size);
	*(ServerCmd *)packet->data() = (ServerCmd)tobig16((u16)cmd);

	// Fill packet data
	index = 0;
	u32 offset = 0;
	while(true) {
		auto *datasize = datas[index];
		if(datasize) {
			memcpy(packet->data() + sizeof(ServerCmd) + offset, datasize->data, datasize->size);
			offset += datasize->size;
			index++;
		} else {
			break;
		}
	}

	return packet;
}

Packet preparePacket(ServerCmd cmd, const void *data, u32 size) {
	Datasize datasize(data, size);
	Datasize *datas[] = {&datasize, nullptr};
	return preparePacket(cmd, datas);
}

Packet preparePacketUserCursorPos(SessionID session_id, s32 x, s32 y) {
	struct PACKED {
		u16 id;
		s32 x;
		s32 y;
	} data;

	data.id = tobig16(session_id.get());
	data.x = tobig32(x);
	data.y = tobig32(y);

	return preparePacket(ServerCmd::user_cursor_pos, &data, sizeof(data));
}

Packet preparePacketUserCreate(Session *session) {
	u16 id_BE = tobig16(session->getID()->get());
	auto nickname = session->getNickname();

	Buffer buf;
	buf.write(&id_BE, sizeof(id_BE));
	buf.write(nickname.data(), nickname.size());

	return preparePacket(ServerCmd::user_create, buf.data(), buf.size());
}

Packet preparePacketUserRemove(Session *session) {
	u16 id_BE = tobig16(session->getID()->get());
	return preparePacket(ServerCmd::user_remove, &id_BE, sizeof(id_BE));
}

Packet preparePacketChunkCreate(Int2 chunk_pos) {
	Int2 chunk_pos_BE;
	chunk_pos_BE.x = tobig32(chunk_pos.x);
	chunk_pos_BE.y = tobig32(chunk_pos.y);
	return preparePacket(ServerCmd::chunk_create, &chunk_pos_BE, sizeof(chunk_pos_BE));
}

Packet preparePacketChunkRemove(Int2 chunk_pos) {
	Int2 chunk_pos_BE;
	chunk_pos_BE.x = tobig32(chunk_pos.x);
	chunk_pos_BE.y = tobig32(chunk_pos.y);
	return preparePacket(ServerCmd::chunk_remove, &chunk_pos_BE, sizeof(chunk_pos_BE));
}

Packet preparePacketMessage(MessageType type, const char *message) {
	Buffer buf;
	buf.write(&type, 1);
	buf.write(message, strlen(message));
	return preparePacket(ServerCmd::message, buf.data(), buf.size());
}

SharedVector<u8> compressLZ4(const void *data, u32 raw_size) NO_SANITIZER {
	auto max_dst_size = LZ4_compressBound(raw_size);
	auto compressed = createSharedVector<u8>(max_dst_size);
	auto compressed_data_size = LZ4_compress_HC((const char *)data, (char *)compressed->data(), raw_size, max_dst_size, LZ4HC_CLEVEL_MAX);
	assert(compressed_data_size > 0);
	compressed->resize(compressed_data_size);
	compressed->shrink_to_fit();
	return compressed;
}

int decompressLZ4(const void *compressed_data, u32 compressed_size, void *raw_data, u32 raw_size) NO_SANITIZER {
	return LZ4_decompress_safe((const char *)compressed_data, (char *)raw_data, compressed_size, raw_size);
}