'use client';

import { useEffect, useState, useRef, useCallback } from 'react';

export function useEngine() {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState(null);
  const workerRef = useRef(null);
  const requestIdRef = useRef(0);
  const pendingRef = useRef(new Map());

  // Send a message to the worker and wait for the response
  const send = useCallback((type, payload = {}) => {
    return new Promise((resolve, reject) => {
      const id = ++requestIdRef.current;
      pendingRef.current.set(id, { resolve, reject });
      workerRef.current?.postMessage({ id, type, payload });
    });
  }, []);

  // Initialize worker on mount, clean up on unmount
  useEffect(() => {
    const worker = new Worker('/wasm/vantage.worker.js', { type: 'module' });
    workerRef.current = worker;

    worker.onmessage = (e) => {
      const { id, type, payload, error } = e.data;
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

    worker.onerror = (e) => {
      console.error('Worker error:', e);
      setError(e.message);
    };

    // Init the WASM engine inside the worker
    send('init')
      .then(() => setIsLoading(false))
      .catch((err) => {
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

  // Engine API — each method returns a Promise
  const engine = {
    newGame: () => send('new_game'),

    setPosition: (fen) => send('set_position', { fen }),

    setPositionStartpos: (moves = '') => send('set_position', { moves }),

    makeMove: (uci) => send('apply_move', { move: uci }),

    isMoveLegal: async (uci) => {
      const result = await send('is_move_legal', { move: uci });
      return result.legal;
    },

    search: (options = {}) => send('search', options),

    getLegalMoves: async () => {
      const result = await send('get_legal_moves');
      return result.moves;
    },

    getLegalMovesForSquare: async (square) => {
      const result = await send('get_legal_moves_for_square', { square });
      return result.moves;
    },

    getFen: async () => {
      const result = await send('get_fen');
      return result.fen;
    },

    getGameStatus: async () => {
      const result = await send('get_game_status');
      return result.status;
    },

    getSideToMove: async () => {
      const result = await send('side_to_move');
      return result.side;
    },
  };

  return { engine, isLoading, error };
}