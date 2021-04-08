#include "types.hpp"
#include <math.h>

Vec2 &Vec2::operator=(const Vec2 &v) {
	x = v.x;
	y = v.y;
	return *this;
}

bool Vec2::operator==(Vec2 v) {
	return x == v.x && y == v.y;
}
bool Vec2::operator!=(Vec2 v) {
	return x != v.x || y != v.y;
}

Vec2 Vec2::operator+(const Vec2 &v) {
	return Vec2(x + v.x, y + v.y);
}

Vec2 Vec2::operator-(const Vec2 &v) {
	return Vec2(x - v.x, y - v.y);
}

Vec2 Vec2::operator/(const Vec2 &v) {
	return Vec2(x / v.x, y / v.y);
}

Vec2 Vec2::operator-() {
	return Vec2(-x, -y);
}

Vec2 &Vec2::operator+=(const Vec2 &v) {
	x += v.x;
	y += v.y;
	return *this;
}

Vec2 &Vec2::operator-=(const Vec2 &v) {
	x -= v.x;
	y -= v.y;
	return *this;
}

Vec2 Vec2::operator+(float s) {
	return Vec2(x + s, y + s);
}

Vec2 Vec2::operator-(float s) {
	return Vec2(x - s, y - s);
}

Vec2 Vec2::operator*(float s) {
	return Vec2(x * s, y * s);
}

Vec2 Vec2::operator/(float s) {
	return Vec2(x / s, y / s);
}

Vec2 &Vec2::operator+=(float s) {
	x += s;
	y += s;
	return *this;
}

Vec2 &Vec2::operator-=(float s) {
	x -= s;
	y -= s;
	return *this;
}

Vec2 &Vec2::operator*=(float s) {
	x *= s;
	y *= s;
	return *this;
}

Vec2 &Vec2::operator/=(float s) {
	x /= s;
	y /= s;
	return *this;
}

void Vec3::normalize() {
	float w = sqrt(x * x + y * y + z * z);
	x /= w;
	y /= w;
	z /= w;
}

Vec3 normalize(const Vec3 &in) {
	float w = sqrt(in.x * in.x + in.y * in.y + in.z * in.z);
	return Vec3(in.x / w, in.y / w, in.z / w);
}

Vec3::Vec3(float x, float y, float z) {
	this->x = x;
	this->y = y;
	this->z = z;
}

Vec3 &Vec3::operator=(const Vec3 &v) {
	this->x = v.x;
	this->y = v.y;
	this->z = v.z;
	return *this;
}

bool Vec3::operator==(Vec3 v) {
	return this->x == v.x && this->y == v.y && this->z == v.z;
}

bool Vec3::operator!=(Vec3 v) {
	return !(this->x == v.x && this->y == v.y && this->z == v.z);
}

Vec3 Vec3::operator+(const Vec3 &v) {
	return {this->x + v.x, this->y + v.y, this->z + v.z};
}

Vec3 Vec3::operator-(const Vec3 &v) {
	return {this->x - v.x, this->y - v.y, this->z - v.z};
}

Vec3 Vec3::operator-() {
	return {-this->x, -this->y, -this->z};
}

Vec3 &Vec3::operator+=(const Vec3 &v) {
	this->x += v.x;
	this->y += v.y;
	this->z += v.z;
	return *this;
}

Vec3 &Vec3::operator-=(const Vec3 &v) {
	this->x -= v.x;
	this->y -= v.y;
	this->z -= v.z;
	return *this;
}

Vec3 Vec3::operator+(float s) {
	return {this->x + s, this->y + s, this->z + s};
}

Vec3 Vec3::operator-(float s) {
	return {this->x - s, this->y - s, this->z - s};
}

Vec3 Vec3::operator*(float s) {
	return {this->x * s, this->y * s, this->z * s};
}

Vec3 Vec3::operator/(float s) {
	return {this->x / s, this->y / s, this->z / s};
}

Vec3 &Vec3::operator+=(float s) {
	this->x += s;
	this->y += s;
	this->z += s;
	return *this;
}

Vec3 &Vec3::operator-=(float s) {
	this->x -= s;
	this->y -= s;
	this->z -= s;
	return *this;
}

Vec3 &Vec3::operator*=(float s) {
	this->x *= s;
	this->y *= s;
	this->z *= s;
	return *this;
}

Vec3 &Vec3::operator/=(float s) {
	this->x /= s;
	this->y /= s;
	this->z /= s;
	return *this;
}

Vec4::Vec4(float x, float y, float z, float w) {
	this->x = x;
	this->y = y;
	this->z = z;
	this->w = w;
}

float VecDistance(const Vec2 &p1, const Vec2 &p2) {
	return sqrtf(powf(p1.x - p2.x, 2) + powf(p1.y - p2.y, 2));
}

float VecAngle(const Vec2 p1, const Vec2 p2) {
	return atan2f(p1.y - p2.y, p1.x - p2.x);
}

Vec2 VecStep(const float angle) {
	return {sin(angle), cos(angle)};
}

float VecDistance(const Vec3 &p1, const Vec3 &p2) {
	return sqrtf(powf(p1.x - p2.x, 2) + powf(p1.y - p2.y, 2) + powf(p1.z - p2.z, 2));
}

float VecDistance(const Vec2 &p) {
	return sqrtf(powf(p.x, 2.0f) + powf(p.y, 2.0f));
}

Vec4 VecBoundary(const Vec2 a, const Vec2 b) {
	if(a.x < b.x) {
		if(a.y < b.y) {
			return {a.x, a.y, b.x - a.x, b.y - a.y};
		} else {
			return {a.x, b.y, b.x - a.x, a.y - b.y};
		}
	} else {
		if(a.y < b.y) {
			return {b.x, a.y, a.x - b.x, b.y - a.y};
		} else {
			return {b.x, b.y, a.x - b.x, a.y - b.y};
		}
	}
}

bool VecTouching(const Vec2 pos1, const Vec2 size1, const Vec2 pos2, const Vec2 size2) {
	return pos1.x + size1.x > pos2.x && pos1.y + size1.y > pos2.y && pos1.x < pos2.x + size2.x && pos1.y < pos2.y + size2.y;
}

bool InRectangle(const Vec2 point, const Vec2 position, const Vec2 size) {
	return point.x >= position.x && point.x < position.x + size.x && point.y >= position.y && point.y < position.y + size.y;
}