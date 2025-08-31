import type { FC, ReactNode } from "react";
import style from "@/style.module.scss"

interface FloatContainerProps {
	children: ReactNode;
	className?: string;
}

export const FloatContainer: FC<FloatContainerProps> = ({ children, className }) => (
	<div className={"" + style.float_container + " " + (className ?? "")}>
		{children}
	</div>
);
