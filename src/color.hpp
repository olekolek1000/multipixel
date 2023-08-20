#pragma once

#include "util/types.hpp"

struct Color {
	u8 r, g, b;

	Color(u8 r, u8 g, u8 b)
			: r(r), g(g), b(b) {
	}

	Color() = default;

	bool operator==(const Color &other) const {
		return r == other.r && g == other.g && b == other.b;
	}

	bool operator!=(const Color &other) const {
		return !(r == other.r && g == other.g && b == other.b);
	}
};