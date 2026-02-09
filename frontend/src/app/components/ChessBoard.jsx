'use client';

import { useState } from 'react';
import { Chess } from 'chess.js';
import { Chessboard } from 'react-chessboard';
import { useEngine } from '../hooks/useEngine';

function GameOverModal({ status, onNewGame }) {
    const isCheckmate = status === 'checkmate';

    const messages = {
        checkmate: 'Checkmate',
        stalemate: 'Stalemate',
        draw_threefold: 'Threefold Repetition',
        draw_50move: '50 Move Rule',
        draw_fivefold: 'Fivefold Repetition',
        draw_75move: '75 Move Rule',
        draw_dead: 'Insufficient Material',
    };

    const subtitle = isCheckmate ? 'The engine wins.' : 'The game is a draw.';

    return (
        <div style={{
            position: 'fixed', inset: 0,
            background: 'rgba(0, 0, 0, 0.75)',
            backdropFilter: 'blur(8px)',
            WebkitBackdropFilter: 'blur(8px)',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            zIndex: 1000,
            animation: 'fadeIn 0.3s ease-out',
        }}>
            <div style={{
                background: 'linear-gradient(145deg, #1a1a1a 0%, #111111 100%)',
                border: '1px solid rgba(255, 255, 255, 0.08)',
                padding: '48px 40px',
                borderRadius: '16px',
                textAlign: 'center',
                minWidth: '320px',
                maxWidth: '400px',
                boxShadow: '0 25px 60px rgba(0, 0, 0, 0.6), 0 0 40px rgba(0, 0, 0, 0.3)',
                animation: 'slideUp 0.4s cubic-bezier(0.16, 1, 0.3, 1)',
            }}>
                {/* Chess icon */}
                <div style={{
                    fontSize: '48px',
                    marginBottom: '20px',
                    filter: 'drop-shadow(0 4px 8px rgba(0,0,0,0.3))',
                }}>
                    {isCheckmate ? '♚' : '½'}
                </div>

                {/* Title */}
                <h2 style={{
                    fontSize: '28px',
                    fontWeight: 700,
                    color: '#ffffff',
                    marginBottom: '8px',
                    letterSpacing: '-0.5px',
                }}>
                    {messages[status] || 'Game Over'}
                </h2>

                {/* Subtitle */}
                <p style={{
                    fontSize: '15px',
                    color: 'rgba(255, 255, 255, 0.5)',
                    marginBottom: '32px',
                    fontWeight: 400,
                }}>
                    {subtitle}
                </p>

                {/* Divider */}
                <div style={{
                    height: '1px',
                    background: 'linear-gradient(90deg, transparent, rgba(255,255,255,0.1), transparent)',
                    marginBottom: '28px',
                }} />

                {/* New Game button */}
                <button
                    onClick={onNewGame}
                    style={{
                        padding: '12px 36px',
                        fontSize: '15px',
                        fontWeight: 600,
                        background: 'rgba(255, 255, 255, 0.9)',
                        color: '#0a0a0a',
                        border: 'none',
                        borderRadius: '10px',
                        cursor: 'pointer',
                        transition: 'all 0.2s ease',
                        letterSpacing: '0.3px',
                    }}
                    onMouseEnter={(e) => {
                        e.target.style.background = '#ffffff';
                        e.target.style.transform = 'translateY(-1px)';
                        e.target.style.boxShadow = '0 4px 12px rgba(255, 255, 255, 0.15)';
                    }}
                    onMouseLeave={(e) => {
                        e.target.style.background = 'rgba(255, 255, 255, 0.9)';
                        e.target.style.transform = 'translateY(0)';
                        e.target.style.boxShadow = 'none';
                    }}
                >
                    New Game
                </button>
            </div>

            <style>{`
                @keyframes fadeIn {
                    from { opacity: 0; }
                    to { opacity: 1; }
                }
                @keyframes slideUp {
                    from { opacity: 0; transform: translateY(20px) scale(0.96); }
                    to { opacity: 1; transform: translateY(0) scale(1); }
                }
            `}</style>
        </div>
    );
}

export default function ChessBoard() {
    const [game] = useState(new Chess());
    const [position, setPosition] = useState('start');
    const [gameOver, setGameOver] = useState(null);
    const [isThinking, setIsThinking] = useState(false);
    const [lastMove, setLastMove] = useState(null);
    const { engine, isLoading, error } = useEngine();

    if (isLoading) {
        return <div style={{ padding: '20px', textAlign: 'center' }}>Loading chess engine...</div>;
    }

    if (error) {
        return <div style={{ padding: '20px', color: 'red' }}>Error: {error}</div>;
    }

    function onPieceDrop(sourceSquare, targetSquare, piece) {
        if (gameOver || isThinking) return false;

        const move = game.move({
            from: sourceSquare,
            to: targetSquare,
            promotion: 'q',
        });

        if (!move) return false;

        setPosition(game.fen());

        const uci = sourceSquare + targetSquare;
        handleMove(uci);
        return true;
    }

    async function handleMove(uci) {
        const result = await engine.makeMove(uci);

        if (result.status !== 'active') {
            setLastMove({ from: uci.substring(0, 2), to: uci.substring(2, 4) });
            setGameOver(result.status);
            return;
        }

        setIsThinking(true);
        const searchResult = await engine.search({ depth: 10 });
        const engineResult = await engine.makeMove(searchResult.bestmove);
        setIsThinking(false);

        game.move(searchResult.bestmove, { sloppy: true });

        setPosition(engineResult.fen);
        setLastMove({ from: searchResult.bestmove.substring(0, 2), to: searchResult.bestmove.substring(2, 4) });

        if (engineResult.status !== 'active') {
            setGameOver(engineResult.status);
        }
    }

    async function handleNewGame() {
        await engine.newGame();
        game.reset();
        setPosition('start');
        setGameOver(null);
        setLastMove(null);
    }

    const highlightStyles = {};
    if (lastMove) {
        const color = 'rgba(130, 100, 60, 0.5)';
        highlightStyles[lastMove.from] = { background: color };
        highlightStyles[lastMove.to] = { background: color };
    }

    return (
        <div style={{
            maxWidth: '600px',
            margin: '0 auto',
            padding: '40px 20px',
            color: 'white',
            fontFamily: 'system-ui, -apple-system, sans-serif',
        }}>
            <h1 style={{
                fontSize: '28px',
                fontWeight: 700,
                marginBottom: '24px',
                color: 'white',
            }}>
                Vantage Chess
            </h1>
            <Chessboard
                position={position}
                onPieceDrop={onPieceDrop}
                customSquareStyles={highlightStyles}
                animationDuration={150}
                customDarkSquareStyle={{ backgroundColor: '#4a4a4a' }}
                customLightSquareStyle={{ backgroundColor: '#6b6b6b' }}
                customBoardStyle={{
                    borderRadius: '8px',
                    boxShadow: '0 4px 20px rgba(0, 0, 0, 0.5)',
                }}
            />
            {gameOver && (
                <GameOverModal status={gameOver} onNewGame={handleNewGame} />
            )}
        </div>
    );
}