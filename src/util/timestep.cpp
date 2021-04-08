#include "timestep.hpp"
#include <algorithm>

float lerp(float alpha, float prev, float var) {
	return var * alpha + prev * (1.0f - alpha);
}

Vec2 lerp(float alpha, Vec2 prev, Vec2 var) {
	return {var.x * alpha + prev.x * (1.0f - alpha), var.y * alpha + prev.y * (1.0f - alpha)};
}

Vec3 lerp(float alpha, Vec3 prev, Vec3 var) {
	return {var.x * alpha + prev.x * (1.0f - alpha), var.y * alpha + prev.y * (1.0f - alpha), var.z * alpha + prev.z * (1.0f - alpha)};
}

Vec4 lerp(float alpha, Vec4 prev, Vec4 var) {
	return {var.x * alpha + prev.x * (1.0f - alpha), var.y * alpha + prev.y * (1.0f - alpha), var.z * alpha + prev.z * (1.0f - alpha), var.w * alpha + prev.w * (1.0f - alpha)};
}

void Timestep::calculateAlpha() {
	alpha = std::clamp(accumulator / delta, 0.0f, 1.0f);
}

Timestep::Timestep() {
	reset();
}

void Timestep::setDelta(float n) {
	delta = n;
}

void Timestep::setRate(float n) {
	setDelta(1000.0f / n);
}

float &Timestep::getAlpha() {
	return alpha;
}

float Timestep::getTimeSeconds() {
	return (double)time_micros / 1000.0 / 1000.0;
}

uint64_t Timestep::getTimeMicros() {
	return time_micros;
}

uint32_t Timestep::getTimeMillis() {
	return time_micros / 1000;
}

void Timestep::setSpeed(float n) {
	speed = n;
}

float Timestep::getSpeed() {
	return speed;
}

bool Timestep::onTick() {
	auto newtime = std::chrono::high_resolution_clock::now();

	uint32_t frametime = std::chrono::duration_cast<std::chrono::microseconds>(newtime - currenttime).count();

	time_micros += frametime;

	currenttime = newtime;

	accumulator += frametime * speed / 1000.0;
	calculateAlpha();

	if(accumulator >= delta) {
		accumulator -= delta;
		loopnum++;
		ticks++;

		if(loopnum > 3) { //cannot keep up!
			loopnum = 0;
			accumulator = 0.0f;
			return false;
		}

		return true;
	} else {
		loopnum = 0;
		return false;
	}
}
void Timestep::reset() {
	currenttime = std::chrono::high_resolution_clock::now();
	accumulator = 0.0f;
}

uint32_t Timestep::getTicks() {
	return ticks;
}