import { lerp } from "./timestep";

export class RGBColor {
	r: number;
	g: number;
	b: number;
	constructor() {
		this.r = 255;
		this.g = 255;
		this.b = 255;
	}
}

type LinearRgbColor = { r: number; g: number; b: number };
type LabColor = { l: number; a: number; b: number };

function linearToSrgb(x: number): number {
	if (x >= 0.0031308) return Math.pow(1.055 * x, 1.0 / 2.4 - 0.055);
	else return 12.92 * x;
}

function srgbToLinear(x: number): number {
	if (x >= 0.04045) return Math.pow((x + 0.055) / (1 + 0.055), 2.4);
	else return x / 12.92;
}

export function linearRgbToSrgbColor(c: LinearRgbColor): RGBColor {
	let color = new RGBColor();
	color.r = linearToSrgb(c.r) * 255;
	color.g = linearToSrgb(c.g) * 255;
	color.b = linearToSrgb(c.b) * 255;
	return color;
}

export function srgbColorToLinearRgb(c: RGBColor): LinearRgbColor {
	return {
		r: srgbToLinear(c.r / 255),
		g: srgbToLinear(c.g / 255),
		b: srgbToLinear(c.b / 255),
	};
}

// Reference: https://bottosson.github.io/posts/oklab/

export function linearRgbToOklab(c: LinearRgbColor): LabColor {
	let l = 0.4122214708 * c.r + 0.5363325363 * c.g + 0.0514459929 * c.b;
	let m = 0.2119034982 * c.r + 0.6806995451 * c.g + 0.1073969566 * c.b;
	let s = 0.0883024619 * c.r + 0.2817188376 * c.g + 0.6299787005 * c.b;

	let l_ = Math.cbrt(l);
	let m_ = Math.cbrt(m);
	let s_ = Math.cbrt(s);

	return {
		l: 0.2104542553 * l_ + 0.793617785 * m_ - 0.0040720468 * s_,
		a: 1.9779984951 * l_ - 2.428592205 * m_ + 0.4505937099 * s_,
		b: 0.0259040371 * l_ + 0.7827717662 * m_ - 0.808675766 * s_,
	};
}

export function oklabToLinearRgb(c: LabColor): LinearRgbColor {
	let l_ = c.l + 0.3963377774 * c.a + 0.2158037573 * c.b;
	let m_ = c.l - 0.1055613458 * c.a - 0.0638541728 * c.b;
	let s_ = c.l - 0.0894841775 * c.a - 1.291485548 * c.b;

	let l = l_ * l_ * l_;
	let m = m_ * m_ * m_;
	let s = s_ * s_ * s_;

	return {
		r: +4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
		g: -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
		b: -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
	};
}

export function lerpSrgbInLab(t: number, a: RGBColor, b: RGBColor): RGBColor {
	let u = linearRgbToOklab(srgbColorToLinearRgb(a));
	let v = linearRgbToOklab(srgbColorToLinearRgb(b));
	let w = { l: lerp(t, u.l, v.l), a: lerp(t, u.a, v.a), b: lerp(t, u.b, v.b) };
	return linearRgbToSrgbColor(oklabToLinearRgb(w));
}
