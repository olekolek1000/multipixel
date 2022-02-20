#include "preview_system.hpp"
#include "chunk_system.hpp"
#include "command.hpp"
#include "room.hpp"
#include "util/mutex.hpp"
#include <cassert>

// static const char *LOG_PREVIEW_SYSTEM = "PreviewSystem";
static const char *LOG_PREVIEW_SYSTEM_LAYER = "PreviewSystemLayer";

PreviewSystemLayer::PreviewSystemLayer(PreviewSystem *system)
		: system(system) {
}

void PreviewSystemLayer::addToQueue(Int2 coords) {
	// Check if already added to queue (reverse iterator)
	for(auto it = update_queue.rbegin(); it != update_queue.rend(); it++) {
		if(*it == coords)
			return;
	}

	// Add to queue
	update_queue.push_back(coords);
}

bool PreviewSystemLayer::processOneBlock() {
	if(update_queue.empty())
		return false;

	auto position = update_queue.front();
	update_queue.pop_front();

	Int2 topleft = {position.x * 2, position.y * 2};
	Int2 topright = {position.x * 2 + 1, position.y * 2};
	Int2 bottomleft = {position.x * 2, position.y * 2 + 1};
	Int2 bottomright = {position.x * 2 + 1, position.y * 2 + 1};

	SharedVector<u8> compressed_topleft, compressed_topright, compressed_bottomleft, compressed_bottomright; // Compressed data

	// Lock database and fetch compressed data
	auto &database = system->room->database;
	database.lock();
	if(zoom == 1) { // Real chunks underneath
		compressed_topleft = database.chunkLoadData(topleft).data;
		compressed_topright = database.chunkLoadData(topright).data;
		compressed_bottomleft = database.chunkLoadData(bottomleft).data;
		compressed_bottomright = database.chunkLoadData(bottomright).data;
	} else {
		compressed_topleft = database.previewLoadData(topleft, zoom - 1).data;
		compressed_topright = database.previewLoadData(topright, zoom - 1).data;
		compressed_bottomleft = database.previewLoadData(bottomleft, zoom - 1).data;
		compressed_bottomright = database.previewLoadData(bottomright, zoom - 1).data;
	}

	// Unlock database
	database.unlock();

	const auto chunk_size = ChunkSystem::getChunkSize();

	// Decompress data
	auto decompress = [chunk_size](SharedVector<u8> &in) -> SharedVector<u8> {
		if(!in) return {};
		auto out = createSharedVector<u8>(chunk_size * chunk_size * 3);
		auto ret = decompressLZ4(in->data(), in->size(), out->data(), out->size());
		assert(ret >= 0);
		(void)ret;
		return out;
	};

	auto data_topleft = decompress(compressed_topleft);
	auto data_topright = decompress(compressed_topright);
	auto data_bottomleft = decompress(compressed_bottomleft);
	auto data_bottomright = decompress(compressed_bottomright);

	// Fuse 2x2 chunks into one image
	const auto image_size = chunk_size * 2;
	uniqdata<u8> rgb(image_size * image_size /* 512² */ * 3 /*RGB*/);
	memset(rgb.data(), 255, rgb.size());

	auto fillImage = [&](SharedVector<u8> &data, u8 x, u8 y) {
		if(!data) return;

		u32 offset_x = chunk_size * x;
		u32 offset_y = chunk_size * y;

		const auto *rgb_in = data->data();
		u32 pitch_in = chunk_size * 3;
		u32 pitch_out = image_size * 3;

		for(u32 local_y = 0; local_y < chunk_size; local_y++) {
			for(u32 local_x = 0; local_x < chunk_size; local_x++) {
				u32 target_x = offset_x + local_x;
				u32 target_y = offset_y + local_y;

				u32 offset_in = local_y * pitch_in + local_x * 3;
				u32 offset_out = target_y * pitch_out + target_x * 3;

				rgb[offset_out + 0] = rgb_in[offset_in + 0];
				rgb[offset_out + 1] = rgb_in[offset_in + 1];
				rgb[offset_out + 2] = rgb_in[offset_in + 2];
			}
		}
	};

	// Blit images
	fillImage(data_topleft, 0, 0);
	fillImage(data_topright, 1, 0);
	fillImage(data_bottomleft, 0, 1);
	fillImage(data_bottomright, 1, 1);

	// Downscale image
	uniqdata<u8> downscaled(chunk_size * chunk_size * 3);
	const u32 downscaled_pitch = chunk_size * 3;
	const u32 image_pitch = image_size * 3;
	for(u32 y = 0; y < chunk_size; y++) {
		for(u32 x = 0; x < chunk_size; x++) {
			u32 in_x = x * 2;
			u32 in_y = y * 2;

			auto *out = &downscaled[y * downscaled_pitch + x * 3];

			auto performChannel = [&](u8 channel) {
				out[channel] =
						((u32)rgb[(in_y + 0) * image_pitch + (in_x + 0) * 3 + channel] +
						 (u32)rgb[(in_y + 1) * image_pitch + (in_x + 0) * 3 + channel] +
						 (u32)rgb[(in_y + 0) * image_pitch + (in_x + 1) * 3 + channel] +
						 (u32)rgb[(in_y + 1) * image_pitch + (in_x + 1) * 3 + channel]) /
						4;
			};

			performChannel(0); // Red
			performChannel(1); // Green
			performChannel(2); // Blue
		}
	}

	// Compress downscaled image
	auto compressed = compressLZ4(downscaled.data(), downscaled.size());

	// Write result, Lock database again
	database.lock();
	database.previewSaveData(position, zoom, compressed->data(), compressed->size());
	// Unlock database
	database.unlock();

	if(upper_layer) {
		s32 x = position.x / 2;
		s32 y = position.y / 2;
		if(position.x < 0) x--;
		if(position.y < 0) y--;
		upper_layer->addToQueue({x, y});
	}

	system->room->log(LOG_PREVIEW_SYSTEM_LAYER, "Processed block at zoom %u (%u remaining)", zoom, (u32)update_queue.size());
	return true;
}

PreviewSystem::PreviewSystem(Room *room)
		: room(room) {
	// Generate layers
	layers.resize(getLayerCount(), this);
	for(u32 i = 0; i < getLayerCount(); i++) {
		auto &layer = layers[i];
		layer.zoom = layerIndexToZoom(i);
		if(i < getLayerCount() - 1)
			layer.upper_layer = &layers[i + 1];
	}
}

void PreviewSystem::tick() {
	{
		LockGuard lock(mtx_access);
		for(auto &pos : update_queue_cache) {
			layers[0].addToQueue(pos);
		}
		update_queue_cache.clear();
	}

	for(auto &layer : layers) {
		if(!layer.processOneBlock())
			continue;
		break;
	}

	queue.process();
}

void PreviewSystem::addToQueueFront(Int2 coords) {
	LockGuard lock(mtx_access);
	update_queue_cache.push_back(coords);
}

PreviewSystem::~PreviewSystem() {
}

u32 PreviewSystem::getLayerCount() const {
	return 4;
}

SharedVector<u8> PreviewSystem::requestData(s32 preview_x, s32 preview_y, u8 zoom) {
	auto &database = room->database;
	database.lock();
	auto data = database.previewLoadData({preview_x, preview_y}, zoom).data;
	database.unlock();
	return data;
}
