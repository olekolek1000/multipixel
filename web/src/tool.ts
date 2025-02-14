export namespace tool {
	export enum ToolID {
		Brush = 0,
		Floodfill = 1,
		Spray = 2,
	}

	export function supportsSmoothing(id: ToolID) {
		switch (id) {
			case ToolID.Brush:
			case ToolID.Spray: {
				return true;
			}
		}
		return false;
	}
}

export default tool;