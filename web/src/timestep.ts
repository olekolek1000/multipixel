export function lerp(alpha: number, prev: number, num: number) {
	return num * alpha + prev * (1.0 - alpha);
}

export class Timestep {
	ticks: number;
	current_time_ms: number;
	time_millis: number;
	accumulator: number;
	delta: number;
	alpha: number;
	speed: number;

	constructor() {
		this.ticks = 0;
		this.current_time_ms = new Date().getTime();
		this.time_millis = 0;
		this.accumulator = 0.0;
		this.delta = 0.0;
		this.alpha = 0.0;
		this.speed = 1.0;
	}

	calculateAlpha = function () {
		let num = this.accumulator / this.delta;
		if (num < 0.0) num = 0.0;
		if (num > 1.0) num = 1.0;
		this.alpha = num;
	}

	setDelta = function (delta: any) {
		this.delta = delta;
	}

	setRate = function (rate: number) {
		this.setDelta(1000.0 / rate);
	}

	getAlpha = function () {
		return this.alpha;
	}

	getTimeMillis = function () {
		return this.time_millis;
	}

	setSpeed = function (speed: any) {
		this.speed = speed;
	}

	getSpeed = function () {
		return this.speed;
	}

	onTick = function () {
		let cur_time = new Date().getTime();
		let frametime = cur_time - this.current_time_ms;
		this.time_millis += frametime;
		this.current_time_ms = cur_time;

		this.accumulator += frametime * this.speed;
		this.calculateAlpha();

		if (this.accumulator >= this.delta) {
			this.accumulator -= this.delta;
			this.loopnum++;
			this.ticks++;

			if (this.loopnum > 20) {//Cannot keep up
				this.loopnum = 0;
				this.accumulator = 0.0;
				return false;
			}

			return true;
		}
		else {
			this.loopnum = 0;
			return false;
		}
	}

	reset = function () {
		this.current_time_ms = new Date().getTime();
	}

	getTicks = function () {
		return this.ticks;
	}
}