#pragma once

#include "types.hpp"
#include <chrono>
#include <stdint.h>

float lerp(float alpha, float prev, float var);
Vec2 lerp(float alpha, Vec2 prev, Vec2 var);
Vec3 lerp(float alpha, Vec3 prev, Vec3 var);
Vec4 lerp(float alpha, Vec4 prev, Vec4 var);

struct Timestep {
private:
	u32 ticks = 0;
	u8 loopnum = 0;

	std::chrono::time_point<std::chrono::high_resolution_clock> currenttime;
	u64 time_micros = 0;
	float accumulator = 0.0;
	float delta = 0.0f;
	float alpha = 0.0f;
	float speed = 1.0f;

	void calculateAlpha();

public:
	Timestep();
	void setDelta(float n);
	void setRate(float n);
	float &getAlpha();
	float getTimeSeconds();
	u64 getTimeMicros();
	u32 getTimeMillis();
	void setSpeed(float n);
	float getSpeed();
	bool onTick();
	void reset();
	u32 getTicks();
};
