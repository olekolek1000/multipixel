export namespace tool {
	export enum ToolID {
		Brush = 0,
		Floodfill = 1,
		Spray = 2,
		Blur = 3,
		Smudge = 4,
		SmoothBrush = 5,
		SquareBrush = 6,
	}

	export function supportsSmoothing(id: ToolID) {
		switch (id) {
			case ToolID.Brush:
			case ToolID.SmoothBrush:
			case ToolID.SquareBrush:
			case ToolID.Spray:
			case ToolID.Blur:
			case ToolID.Smudge: {
				return true;
			}
		}
		return false;
	}
}

export default tool;