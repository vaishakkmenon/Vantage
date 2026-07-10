'use client';

import { useReducer, useRef, useCallback } from 'react';
import { Chess } from 'chess.js';
import { ChessGameState, PlayerColor, DifficultyLevel, GameStatus, GameWinner, MoveRecord } from '../types';

const STARTPOS_FEN = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';

const initialState: ChessGameState = {
    playerColor: 'white',
    difficulty: 'intermediate',
    boardOrientation: 'white',
    moves: [],
    currentMoveIndex: -1,
    displayFen: STARTPOS_FEN,
    liveFen: STARTPOS_FEN,
    status: 'active',
    winner: null,
    isThinking: false,
    showModal: false,
    currentEval: 0,
    gameKey: 0,
};

type Action =
    | { type: 'START_NEW_GAME'; playerColor: PlayerColor; difficulty: DifficultyLevel }
    | { type: 'ADD_MOVE'; move: MoveRecord }
    | { type: 'SET_STATUS'; status: GameStatus; winner: GameWinner }
    | { type: 'SET_THINKING'; isThinking: boolean }
    | { type: 'SET_EVAL'; score: number }
    | { type: 'NAVIGATE_TO'; index: number }
    | { type: 'FLIP_BOARD' }
    | { type: 'SHOW_MODAL' }
    | { type: 'HIDE_MODAL' };

function reducer(state: ChessGameState, action: Action): ChessGameState {
    switch (action.type) {
        case 'START_NEW_GAME':
            return {
                ...initialState,
                playerColor: action.playerColor,
                difficulty: action.difficulty,
                boardOrientation: action.playerColor,
                gameKey: state.gameKey + 1,
            };

        case 'ADD_MOVE': {
            const moves = [...state.moves, action.move];
            return {
                ...state,
                moves,
                currentMoveIndex: moves.length - 1,
                displayFen: action.move.fen,
                liveFen: action.move.fen,
            };
        }

        case 'SET_STATUS':
            return { ...state, status: action.status, winner: action.winner };

        case 'SET_THINKING':
            return { ...state, isThinking: action.isThinking };

        case 'SET_EVAL':
            return { ...state, currentEval: action.score };

        case 'NAVIGATE_TO':
            return {
                ...state,
                currentMoveIndex: action.index,
                displayFen: action.index === -1 ? STARTPOS_FEN : state.moves[action.index].fen,
            };

        case 'FLIP_BOARD':
            return { ...state, boardOrientation: state.boardOrientation === 'white' ? 'black' : 'white' };

        case 'SHOW_MODAL':
            return { ...state, showModal: true };

        case 'HIDE_MODAL':
            return { ...state, showModal: false };
    }
}

export function useChessGame() {
    const [state, dispatch] = useReducer(reducer, initialState);
    const chessRef = useRef(new Chess());

    const startNewGame = useCallback((playerColor: PlayerColor, difficulty: DifficultyLevel) => {
        chessRef.current.reset();
        dispatch({ type: 'START_NEW_GAME', playerColor, difficulty });
    }, []);

    const applyLocalMove = useCallback((uci: string): { san: string; fen: string } | null => {
        const from = uci.slice(0, 2);
        const to = uci.slice(2, 4);
        const promotion = uci[4];
        try {
            const result = chessRef.current.move({ from, to, promotion });
            return { san: result.san, fen: chessRef.current.fen() };
        } catch {
            return null;
        }
    }, []);

    const addMove = useCallback((move: MoveRecord) => {
        dispatch({ type: 'ADD_MOVE', move });
    }, []);

    const setStatus = useCallback((status: GameStatus, winner: GameWinner) => {
        dispatch({ type: 'SET_STATUS', status, winner });
    }, []);

    const setThinking = useCallback((isThinking: boolean) => {
        dispatch({ type: 'SET_THINKING', isThinking });
    }, []);

    const setEval = useCallback((score: number) => {
        dispatch({ type: 'SET_EVAL', score });
    }, []);

    const navigateTo = useCallback((index: number) => {
        dispatch({ type: 'NAVIGATE_TO', index });
    }, []);

    const flipBoard = useCallback(() => dispatch({ type: 'FLIP_BOARD' }), []);
    const showModal = useCallback(() => dispatch({ type: 'SHOW_MODAL' }), []);
    const hideModal = useCallback(() => dispatch({ type: 'HIDE_MODAL' }), []);

    const isBrowsingHistory = state.currentMoveIndex < state.moves.length - 1;

    return {
        state,
        isBrowsingHistory,
        startNewGame,
        applyLocalMove,
        addMove,
        setStatus,
        setThinking,
        setEval,
        navigateTo,
        flipBoard,
        showModal,
        hideModal,
    };
}