import type { FC, ReactNode } from "react";

type InputProps = React.InputHTMLAttributes<HTMLInputElement> & {
    className?: string;
    children?: ReactNode;
}

export const Input: FC<InputProps> = (props: InputProps) => {
    return (
        <input
            {...props}
            className={"border-2 font-family text-base font-bold bg-white rounded-2xl b-2 border-border/30 p-3 " + (props.className ?? "")}
        />
    );
}