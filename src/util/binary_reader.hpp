#pragma once
#include "types.hpp"
#include <cstddef>
#include <cstring>

struct BinaryReader {
	const void *in_data;
	size_t in_size;
	size_t read_pos = 0;
	inline BinaryReader(const void *in_data, size_t in_size)
			: in_data(in_data),
				in_size(in_size) {
	}

	//Read and move pointer
	inline bool read(void *out, size_t out_size) {
		if(read_pos + out_size > in_size)
			return false;
		memcpy(out, (uint8_t *)in_data + read_pos, out_size);
		read_pos += out_size;
		return true;
	}

	//Read without moving pointer
	inline bool fetch(void *out, size_t out_size) {
		if(read_pos + out_size > in_size)
			return false;
		memcpy(out, (uint8_t *)in_data + read_pos, out_size);
		return true;
	}

	template <typename T>
	inline bool read(T *out) {
		return read(out, sizeof(T));
	}

	template <typename T>
	inline bool fetch(T *out) {
		return fetch(out, sizeof(T));
	}

	inline void *getDataAtReadPos() const {
		return (uint8_t *)in_data + read_pos;
	}

	inline size_t getRemainingSize() const {
		return in_size - read_pos;
	}
};