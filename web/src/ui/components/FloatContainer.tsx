import type { FC, ReactNode } from "react";

interface FloatContainerProps {
    children: ReactNode;
    className?: string;
}

export const FloatContainer: FC<FloatContainerProps> = ({ children, className }) =>  (
    <div className={"bg-background/85 backdrop-blur-lg rounded-2xl shadow-edge-shadow border-4 border-background/30 z-50 " + (className ?? "")}>
        {children}
    </div>
);
