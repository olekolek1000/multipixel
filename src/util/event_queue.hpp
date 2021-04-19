#pragma once

#include <atomic>
#include <functional>
#include <mutex>
#include <queue>
#include <stdint.h>

// Event queue, used for thread-safe, mutltithreaded callback management.
struct EventQueue {
	std::atomic<uint32_t> processing_tasks = 0;
	std::mutex mtx_queue;
	size_t taskIndex = 0;

	// function callback, taskID
	std::deque<std::pair<std::function<void()>, size_t>> queue;

	uint32_t process(uint32_t max_count = UINT32_MAX) {
		uint32_t processed = 0;
		uint32_t i = 0;
		while(true) {
			i++;
			mtx_queue.lock();
			if(!queue.empty()) {
				auto pair = queue.front();
				processing_tasks++;
				queue.pop_front();
				mtx_queue.unlock();
				pair.first(); //Call function
				processing_tasks--;
				processed++;
			} else {
				mtx_queue.unlock();
				break;
			}
			if(i > max_count) {
				break;
			}
		}
		return processed;
	}

	// Returns task ID
	size_t push(std::function<void()> callback) {
		std::lock_guard lock(mtx_queue);
		queue.push_back({callback, taskIndex});
		return taskIndex++;
	}

	bool cancelTask(size_t task) {
		std::lock_guard lock(mtx_queue);
		for(auto it = queue.begin(); it != queue.end(); it++) {
			if(it->second == task) {
				queue.erase(it);
				return true;
			}
		}
		return false;
	}

	void clear() {
		std::lock_guard lock(mtx_queue);
		queue.clear();
	}

	size_t size() {
		std::lock_guard lock(mtx_queue);
		return queue.size();
	}
};