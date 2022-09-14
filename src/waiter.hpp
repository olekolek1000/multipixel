#pragma once
#include <atomic>
#include <condition_variable>
#include <mutex>

struct Waiter {
	std::mutex mtx;
	std::condition_variable cond;
	std::atomic<bool> ready = false;

	std::unique_lock<std::mutex> getLock() {
		ready = false;
		return std::unique_lock(mtx);
	}

	inline void notify() {
		std::unique_lock lk(mtx);
		ready = true;
		cond.notify_one();
	}

	inline void wait(std::unique_lock<std::mutex> &lk) {
		while(!ready) {
			cond.wait(lk);
		}
	}
};