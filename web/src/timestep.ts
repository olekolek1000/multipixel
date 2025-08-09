export function lerp(alpha: number, prev: number, num: number) {
	return num * alpha + prev * (1.0 - alpha);
}

export class Timestep {
	ticks: number = 0;
	current_time_ms: number = new Date().getTime();
	time_millis: number = 0;
	accumulator: number = 0.0;
	delta: number = 0.0;
	alpha: number = 0.0;
	speed: number = 1.0;
	loopnum: number = 0;

	constructor(rate: number) {
		this.setRate(rate);
	}

	calculateAlpha() {
		let num = this.accumulator / this.delta;
		if (num < 0.0) num = 0.0;
		if (num > 1.0) num = 1.0;
		this.alpha = num;
	}

	setDelta(delta: any) {
		this.delta = delta;
	}

	setRate(rate: number) {
		this.setDelta(1000.0 / rate);
	}

	getAlpha() {
		return this.alpha;
	}

	getTimeMillis() {
		return this.time_millis;
	}

	setSpeed(speed: any) {
		this.speed = speed;
	}

	getSpeed() {
		return this.speed;
	}

	onTick() {
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

	reset() {
		this.current_time_ms = new Date().getTime();
	}

	getTicks() {
		return this.ticks;
	}
}