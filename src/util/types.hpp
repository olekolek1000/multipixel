#pragma once

#include <stdint.h>

typedef uint8_t u8;
typedef uint16_t u16;
typedef uint32_t u32;
typedef uint64_t u64;
typedef int8_t s8;
typedef int16_t s16;
typedef int32_t s32;
typedef int64_t s64;

struct Vec2 {
	float x, y;

	Vec2() = default;
	Vec2(float x, float y)
			: x(x), y(y) {}
	Vec2(s32 x, s32 y)
			: x(x), y(y) {}
	Vec2(u32 x, u32 y)
			: x(x), y(y) {}
	Vec2(float n)
			: x(n), y(n) {
	}
	Vec2(const Vec2 &v)
			: x(v.x), y(v.y) {}

	Vec2 &operator=(const Vec2 &v);
	bool operator==(Vec2 v);
	bool operator!=(Vec2 v);
	Vec2 operator+(const Vec2 &v);
	Vec2 operator-(const Vec2 &v);
	Vec2 operator/(const Vec2 &v);
	Vec2 operator-();
	Vec2 &operator+=(const Vec2 &v);
	Vec2 &operator-=(const Vec2 &v);
	Vec2 operator+(float s);
	Vec2 operator-(float s);
	Vec2 operator*(float s);
	Vec2 operator/(float s);
	Vec2 &operator+=(float s);
	Vec2 &operator-=(float s);
	Vec2 &operator*=(float s);
	Vec2 &operator/=(float s);
};

struct Vec3 {
	union {
		float x;
		float r;
	};
	union {
		float y;
		float g;
	};
	union {
		float z;
		float b;
	};

	Vec3() = default;
	Vec3(float x, float y, float z);
	Vec3 &operator=(const Vec3 &v);
	bool operator==(Vec3 v);
	bool operator!=(Vec3 v);
	Vec3 operator+(const Vec3 &v);
	Vec3 operator-(const Vec3 &v);
	Vec3 operator-();
	Vec3 &operator+=(const Vec3 &v);
	Vec3 &operator-=(const Vec3 &v);
	Vec3 operator+(float s);
	Vec3 operator-(float s);
	Vec3 operator*(float s);
	Vec3 operator/(float s);
	Vec3 &operator+=(float s);
	Vec3 &operator-=(float s);
	Vec3 &operator*=(float s);
	Vec3 &operator/=(float s);
	void normalize();
};

Vec3 normalize(const Vec3 &in);

struct Vec4 {
	union {
		float x;
		float r;
	};
	union {
		float y;
		float g;
	};
	union {
		float z;
		float b;
	};
	union {
		float w;
		float a;
	};

	Vec4() = default;
	Vec4(float x, float y, float z, float w);
};

struct Int2 {
	s32 x;
	s32 y;
	Int2() {}
	Int2(s32 n) : x(n), y(n) {}
	Int2(s32 x, s32 y) : x(x), y(y) {}
};

struct UInt2 {
	u32 x;
	u32 y;
};

struct Int3 {
	Int3(int x, int y, int z)
			: x(x), y(y), z(z) {}
	union {
		int x;
		int r;
	};
	union {
		int y;
		int g;
	};
	union {
		int z;
		int b;
	};
};

struct Int4 {
	int x;
	int y;
	int z;
	int w;
};

float VecDistance(const Vec2 &p1, const Vec2 &p2);																			/* Returns distance between 2D points */
float VecDistance(const Vec3 &p1, const Vec3 &p2);																			/* Returns distance between 3D points */
float VecDistance(const Vec2 &p);																												/* Returns distance from 0,0,0 */
float VecAngle(const Vec2 p1, const Vec2 p2);																						/* Returns angle of vector p1->p2 */
Vec2 VecStep(const float angle);																												/* Returns sin(angle), cos(angle) */
Vec4 VecBoundary(const Vec2 a, const Vec2 b);																						/* Boundary of two points. Returns x, y, w, h respectively */
bool VecTouching(const Vec2 pos1, const Vec2 size1, const Vec2 pos2, const Vec2 size2); /* Check if given boundaries are touching */
bool InRectangle(const Vec2 point, const Vec2 position, const Vec2 size);