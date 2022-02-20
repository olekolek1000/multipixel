#pragma once

#include "command.hpp"
#include "util/event_queue.hpp"
#include "util/mutex.hpp"
#include "util/types.hpp"
#include <deque>
#include <functional>
#include <map>
#include <vector>

struct Room;
struct PreviewSystem;

struct PreviewSystemLayer {
	PreviewSystem *system;
	PreviewSystemLayer(PreviewSystem *system);

	uint8_t zoom = 0;
	PreviewSystemLayer *upper_layer = nullptr;

	std::deque<Int2> update_queue;

	void addToQueue(Int2 coords);

	/// @returns true if processed something
	bool processOneBlock();
};

struct PreviewSystem {
	Room *room;
	Mutex mtx_access;

	PreviewSystem(Room *room);
	~PreviewSystem();

	void tick();

	u32 getLayerCount() const;
	inline static u32 layerIndexToZoom(u32 index) {
		return index + 1;
	}

	std::vector<Int2> update_queue_cache;
	void addToQueueFront(Int2 coords);

	SharedVector<u8> requestData(s32 preview_x, s32 preview_y, u8 zoom);

private:
	EventQueue queue;
	std::vector<PreviewSystemLayer> layers;
};