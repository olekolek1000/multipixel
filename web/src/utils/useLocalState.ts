import { useState } from "react";


export const useLocalState = <T,>(
    key: string, 
    initialValue: T
): [T, (value: T) => void] => {
    const [value, setValue] = useState<T>(() => {
        const storedValue = localStorage.getItem(key);
        return storedValue ? JSON.parse(storedValue) : initialValue;
    });

    const setLocalValue = (newValue: T) => {
        setValue(newValue);
        localStorage.setItem(key, JSON.stringify(newValue));
    };

    return [value, setLocalValue];
}