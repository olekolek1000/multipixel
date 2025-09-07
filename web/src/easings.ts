export namespace easings {
	export function cubic(x: number): number {
		return x * x * x;
	}

	export function quint(x: number): number {
		return x * x * x * x * x;
	}

	export function in_out_cubic(x: number): number {
		return x < 0.5 ? 4 * x * x * x : 1 - Math.pow(-2 * x + 2, 3) / 2;
	}

	export function out_cubic(x: number): number {
		return 1 - Math.pow(1 - x, 3);
	}

	export function in_cubic(x: number): number {
		return x * x * x;
	}
}