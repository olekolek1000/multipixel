export namespace tool {
	export enum ToolID {
		Brush = 0,
		Floodfill = 1,
		Spray = 2,
		Blur = 3,
	}

	export function supportsSmoothing(id: ToolID) {
		switch (id) {
			case ToolID.Brush:
			case ToolID.Spray:
			case ToolID.Blur: {
				return true;
			}
		}
		return false;
	}
}

export default tool;