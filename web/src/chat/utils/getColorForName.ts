

// todo: full random colors
//       so every nickname has a unique color

const COLOR_LIST = [
    "#6a4a0d",
    "#2d6099",
    "#8953aa",
    "#267c22",
    "#580d11",
    "#513188"
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
