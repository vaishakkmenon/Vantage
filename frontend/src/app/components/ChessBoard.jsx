'use client';

import { useState } from 'react';
import { Chess } from 'chess.js';
import { Chessboard } from 'react-chessboard';
import { useEngine } from '../hooks/useEngine';
import EvalBar from './EvalBar';
import MoveHistory from './MoveHistory';
import GameOverModal from './GameOverModal'; // Import the extracted component

export default function ChessBoard() {
    const [game] = useState(new Chess());
    const [position, setPosition] = useState('start');
    const [gameOver, setGameOver] = useState(null);
    const [isThinking, setIsThinking] = useState(false);
    const [lastMove, setLastMove] = useState(null);
    const { engine, isLoading, error } = useEngine();

    // Task 1b.1: Track Move History + Eval in State
    const [moves, setMoves] = useState([]);
    const [currentEval, setCurrentEval] = useState(0);

    const currentMoveIndex = moves.length - 1;

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
        handleMove(uci, move.san);
        return true;
    }

    async function handleMove(uci, playerSan) {
        const result = await engine.makeMove(uci);

        setMoves(prev => [...prev, {
            san: playerSan,
            uci: uci,
            fen: result.fen,
            score: null,
            from_book: false,
        }]);

        if (result.status !== 'active') {
            setLastMove({ from: uci.substring(0, 2), to: uci.substring(2, 4) });
            setGameOver(result.status);
            return;
        }

        const engineColor = game.turn();

        setIsThinking(true);
        const searchResult = await engine.search({ depth: 10 });
        const engineResult = await engine.makeMove(searchResult.bestmove);
        setIsThinking(false);

        game.move(searchResult.bestmove, { sloppy: true });

        setPosition(engineResult.fen);
        setLastMove({ from: searchResult.bestmove.substring(0, 2), to: searchResult.bestmove.substring(2, 4) });

        const normalizedScore = engineColor === 'b' ? -searchResult.score : searchResult.score;

        setMoves(prev => [...prev, {
            san: game.history().slice(-1)[0],
            uci: searchResult.bestmove,
            fen: engineResult.fen,
            score: normalizedScore,
            from_book: searchResult.from_book,
        }]);

        setCurrentEval(normalizedScore);

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
        setMoves([]);
        setCurrentEval(0);
    }

    const highlightStyles = {};
    if (lastMove) {
        const color = 'rgba(130, 100, 60, 0.5)';
        highlightStyles[lastMove.from] = { background: color };
        highlightStyles[lastMove.to] = { background: color };
    }

    return (
        <div style={{
            maxWidth: '900px',
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
                textAlign: 'center',
            }}>
                Vantage Chess
            </h1>

            <div style={{
                display: 'flex',
                gap: '24px',
                alignItems: 'stretch',
                justifyContent: 'center',
                height: '560px',
            }}>

                {/* Left: Eval Bar */}
                <EvalBar
                    score={currentEval}
                    isThinking={isThinking}
                    isMate={false}
                />

                {/* Center: Board */}
                <div style={{
                    flexGrow: 1,
                    display: 'flex',
                    alignItems: 'center',
                    maxWidth: '560px',
                }}>
                    <div style={{ width: '100%' }}>
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
                    </div>
                </div>

                {/* Right: Move History Panel */}
                <div style={{
                    width: '240px',
                    background: '#1e1e1e',
                    borderRadius: '8px',
                    boxShadow: '0 4px 12px rgba(0,0,0,0.3)',
                    border: '1px solid #333',
                    display: 'flex',
                    flexDirection: 'column',
                }}>
                    <div style={{
                        padding: '12px 16px',
                        borderBottom: '1px solid #333',
                        background: '#252525',
                        borderTopLeftRadius: '8px',
                        borderTopRightRadius: '8px',
                    }}>
                        <h3 style={{ margin: 0, fontSize: '14px', fontWeight: 600, color: '#ddd' }}>Move History</h3>
                    </div>
                    <div style={{ flex: 1, overflow: 'hidden' }}>
                        <MoveHistory
                            moves={moves}
                            currentMoveIndex={currentMoveIndex}
                            onMoveClick={(index) => console.log('Clicked move:', index)}
                        />
                    </div>
                </div>
            </div>

            {gameOver && (
                <GameOverModal status={gameOver} onNewGame={handleNewGame} />
            )}
        </div>
    );
}