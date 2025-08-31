

// todo: full random colors
//       so every nickname has a unique color

const COLOR_LIST = [
	"#ffbe46ff",
	"#59a6ffff",
	"#cf81ffff",
	"#71ff69ff",
	"#ff5961ff",
	"#9c62ffff"
]

const hashString = (str: string) =>
	str.split('')
		.reduce((hash, char) => {
			hash = char.charCodeAt(0) + ((hash << 5) - hash);
			return hash & hash; // Convert to 32bit integer
		}, 0);

export const getCssColorForName = (str: string): string => {
	const hash = hashString(str);

	const index = Math.abs(hash) % COLOR_LIST.length;

	return COLOR_LIST[index];
}
