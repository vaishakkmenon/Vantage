import { useEffect, useState, useCallback } from 'react';

const MAX_SIZE = 560;
const MIN_SIZE = 260;
const PADDING = 48; // matches ChessGame's p-6 container padding (24px each side)

export function useBoardSize(): [(node: HTMLElement | null) => void, number] {
    const [node, setNode] = useState<HTMLElement | null>(null);
    const [size, setSize] = useState(MAX_SIZE);

    const ref = useCallback((el: HTMLElement | null) => {
        setNode(el);
    }, []);

    useEffect(() => {
        if (!node) return;

        const compute = () => {
            const available = node.clientWidth - PADDING;
            setSize(Math.max(MIN_SIZE, Math.min(MAX_SIZE, available)));
        };

        compute();
        const observer = new ResizeObserver(compute);
        observer.observe(node);
        return () => observer.disconnect();
    }, [node]);

    return [ref, size];
}
