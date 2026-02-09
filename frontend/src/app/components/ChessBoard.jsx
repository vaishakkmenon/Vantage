'use client';

import { useState } from 'react';
import { Chessboard } from 'react-chessboard';
import { useEngine } from '../hooks/useEngine';

function GameOverModal({ status, onNewGame }) {
    const messages = {
        checkmate: 'Checkmate!',
        stalemate: 'Stalemate — Draw',
        draw_threefold: 'Draw — Threefold Repetition',
        draw_50move: 'Draw — 50 Move Rule',
        draw_fivefold: 'Draw — Fivefold Repetition',
        draw_75move: 'Draw — 75 Move Rule',
        draw_dead: 'Draw — Insufficient Material',
    };

    return (
        <div style={{
            position: 'fixed', inset: 0,
            background: 'rgba(0,0,0,0.6)',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            zIndex: 1000,
        }}>
            <div style={{
                background: '#1e1e2e', color: 'white',
                padding: '32px', borderRadius: '12px',
                textAlign: 'center', minWidth: '280px',
            }}>
                <h2 style={{ fontSize: '24px', marginBottom: '16px' }}>
                    {messages[status] || 'Game Over'}
                </h2>
                <button
                    onClick={onNewGame}
                    style={{
                        padding: '10px 24px', fontSize: '16px',
                        background: '#4a9eff', color: 'white',
                        border: 'none', borderRadius: '8px', cursor: 'pointer',
                    }}
                >
                    New Game
                </button>
            </div>
        </div>
    );
}

export default function ChessBoard() {
    const [position, setPosition] = useState('start');
    const [gameOver, setGameOver] = useState(null);
    const { engine, isLoading, error } = useEngine();

    if (isLoading) {
        return <div style={{ padding: '20px', textAlign: 'center' }}>Loading chess engine...</div>;
    }

    if (error) {
        return <div style={{ padding: '20px', color: 'red' }}>Error: {error}</div>;
    }

    function onPieceDrop(sourceSquare, targetSquare, piece) {
        if (gameOver) return false;
        const uci = sourceSquare + targetSquare;
        handleMove(uci);
        return true;
    }

    async function handleMove(uci) {
        const result = await engine.makeMove(uci);
        if (!result.valid) return;

        setPosition(result.fen);

        if (result.status !== 'active') {
            setGameOver(result.status);
            return;
        }

        const searchResult = await engine.search({ depth: 10 });
        const engineResult = await engine.makeMove(searchResult.bestmove);
        setPosition(engineResult.fen);

        if (engineResult.status !== 'active') {
            setGameOver(engineResult.status);
        }
    }

    async function handleNewGame() {
        await engine.newGame();
        setPosition('start');
        setGameOver(null);
    }

    return (
        <div style={{ maxWidth: '600px', margin: '0 auto', padding: '20px' }}>
            <h1>Vantage Chess</h1>
            <Chessboard position={position} onPieceDrop={onPieceDrop} />
            {gameOver && (
                <GameOverModal status={gameOver} onNewGame={handleNewGame} />
            )}
        </div>
    );
}