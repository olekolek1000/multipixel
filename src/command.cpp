#include "command.hpp"
#include "lz4.h"
#include "session.hpp"
#include "util/buffer.hpp"
#include <byteswap.h>
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

	//Count size
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

	//Fill packet data
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

Packet preparePacketUserCursorPos(u16 session_id, s32 x, s32 y) {
	struct PACKED {
		u16 id;
		s32 x;
		s32 y;
	} data;

	data.id = tobig16(session_id);
	data.x = tobig32(x);
	data.y = tobig32(y);

	return preparePacket(ServerCmd::user_cursor_pos, &data, sizeof(data));
}

Packet preparePacketUserCreate(Session *session) {
	u16 id_BE = tobig16(session->getID());
	auto nickname = session->getNickname();

	Buffer buf;
	buf.write(&id_BE, sizeof(id_BE));
	buf.write(nickname.data(), nickname.size());

	return preparePacket(ServerCmd::user_create, buf.data(), buf.size());
}

Packet preparePacketUserRemove(Session *session) {
	u16 id_BE = tobig16(session->getID());
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

uniqdata<u8> compressLZ4(const void *data, u32 raw_size) NO_SANITIZER {
	auto max_dst_size = LZ4_compressBound(raw_size);
	uniqdata<u8> compressed(max_dst_size);
	auto compressed_data_size = LZ4_compress_default((const char *)data, (char *)compressed.data(), raw_size, max_dst_size);
	assert(compressed_data_size > 0);
	compressed.resize(compressed_data_size);
	return compressed;
}

uniqdata<u8> decompressLZ4(const void *data, u32 compressed_size, u32 raw_size) NO_SANITIZER {
	uniqdata<u8> decompressed(raw_size);
	auto ret = LZ4_decompress_safe((const char *)data, (char *)decompressed.data(), compressed_size, raw_size);
	if(ret < 0)
		decompressed.reset();
	return decompressed;
}