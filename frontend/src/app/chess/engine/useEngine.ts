'use client'

import { useEffect, useState, useRef, useCallback } from 'react';
import { EngineAdapter } from './adapter';
import { GameStatus, PlayerColor, SearchOptions } from '../types';

export function useEngine() {
    const [isLoading, setIsLoading] = useState<boolean>(true);
    const [error, setError] = useState<string | null>(null);
    const workerRef = useRef<Worker | null>(null);
    const requestIdRef = useRef<number>(0);
    const pendingRef = useRef<Map<number, { resolve: (value: unknown) => void; reject: (reason: Error) => void }>>(new Map());

    const send = useCallback((type: string, payload: Record<string, unknown> = {}): Promise<any> => {
        return new Promise((resolve, reject) => {
            const id = ++requestIdRef.current;
            pendingRef.current.set(id, { resolve, reject });
            workerRef.current?.postMessage({ id, type, payload });
        });
    }, []);

    useEffect(() => {
        const worker = new Worker('/wasm/vantage.worker.js', { type: 'module' });
        workerRef.current = worker;

        worker.onmessage = (e: MessageEvent<{ id: number; type: string; payload: unknown; error?: string }>) => {
            const { id, payload, error } = e.data;
            const pending = pendingRef.current.get(id);
            if (pending) {
                pendingRef.current.delete(id);
                if (error) {
                    pending.reject(new Error(error));
                } else {
                    pending.resolve(payload);
                }
            }
        };

        worker.onerror = (e: ErrorEvent) => {
            console.error('Worker error:', e);
            setError(e.message || 'Worker failed to load');
            setIsLoading(false);
        };

        send('init')
            .then(() => setIsLoading(false))
            .catch((err: Error) => {
                console.error('Failed to initialize engine:', err);
                setError(err.message);
                setIsLoading(false);
            });

        return () => {
            worker.terminate();
            workerRef.current = null;
            pendingRef.current.clear();
        };
    }, [send]);

    const engine: EngineAdapter = {
        newGame: () => send('new_game'),

        setPosition: (fen: string) => send('set_position', { fen }),

        setPositionStartpos: (moves: string = '') => send('set_position', { moves }),

        makeMove: (uci: string) => send('apply_move', { move: uci }),

        isMoveLegal: async (uci: string) => {
            const result = await send('is_move_legal', { move: uci });
            return result.legal as boolean;
        },

        search: (options: SearchOptions = {}) => send('search', options),

        getLegalMoves: async () => {
            const result = await send('get_legal_moves');
            return result.moves as string[];
        },

        getLegalMovesForSquare: async (square: string) => {
            const result = await send('get_legal_moves_for_square', { square })
            return result.moves as string[];
        },

        getFen: async () => {
            const result = await send('get_fen');
            return result.fen as string;
        },

        getGameStatus: async () => {
            const result = await send('get_game_status');
            return result.status as GameStatus;
        },

        getSideToMove: async () => {
            const result = await send('side_to_move');
            return result.side as PlayerColor;
        },
    };

    return { engine, isLoading, error };
}
