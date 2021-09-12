#pragma once

#include "types.hpp"
#include <cstring>
#include <vector>

struct Buffer {
private:
	std::vector<u8> vec;

public:
	Buffer() {
	}

	u8 *data() {
		return vec.data();
	}

	size_t size() {
		return vec.size();
	}

	void reserve(size_t size) {
		vec.reserve(size);
	}

	void write(const void *data, size_t size) {
		vec.insert(vec.end(), (u8 *)data, (u8 *)data + size);
	}
};