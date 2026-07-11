'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import { useTheme } from 'next-themes';
import { useEngine } from '../engine/useEngine';
import { useChessGame } from '../hooks/useChessGame';
import { useBoardSize } from '../hooks/useBoardSize';
import { DIFFICULTY_CONFIGS } from '../lib/difficulty';
import { ACCENT_COLORS } from '../lib/theme';
import { DifficultyLevel, GameStatus, GameWinner, PlayerColor } from '../types';
import { LoadingScreen } from './LoadingScreen';
import { Board } from './Board';
import { EvalBar } from './EvalBar';
import { MoveHistory } from './MoveHistory';
import { GameControls } from './GameControls';
import { GameOverModal } from './GameOverModal';
import { NewGameDialog } from './NewGameDialog';

function deriveWinner(status: GameStatus, mover: PlayerColor): GameWinner {
    if (status === 'checkmate') return mover;
    if (status === 'active') return null;
    return 'draw'; // stalemate + all draw_* variants
}

export function ChessGame() {
    const { engine, isLoading, error } = useEngine();
    const chess = useChessGame();
    const { resolvedTheme } = useTheme();
    const [showNewGameDialog, setShowNewGameDialog] = useState(true);
    const [containerRef, boardSize] = useBoardSize();

    const [selectedSquare, setSelectedSquare] = useState<string | null>(null);
    const [selectedPiece, setSelectedPiece] = useState<string | null>(null);
    const [legalDestinations, setLegalDestinations] = useState<string[]>([]);
    const selectionKeyRef = useRef(0);

    const clearSelection = useCallback(() => {
        setSelectedSquare(null);
        setSelectedPiece(null);
        setLegalDestinations([]);
        selectionKeyRef.current++;
    }, []);

    const gameKeyRef = useRef(chess.state.gameKey);
    useEffect(() => {
        gameKeyRef.current = chess.state.gameKey;
    }, [chess.state.gameKey]);

    useEffect(() => {
        clearSelection();
    }, [chess.state.gameKey, clearSelection]);

    const runEngineTurn = useCallback(async (difficulty: DifficultyLevel, keyAtStart: number) => {
        chess.setThinking(true);
        try {
            const searchResult = await engine.search(DIFFICULTY_CONFIGS[difficulty].searchOptions);
            if (gameKeyRef.current !== keyAtStart) return; // new game started mid-search, drop this response

            const local = chess.applyLocalMove(searchResult.bestmove);
            const engineResult = await engine.makeMove(searchResult.bestmove);
            if (!local || gameKeyRef.current !== keyAtStart) return;

            const mover: PlayerColor = chess.state.playerColor === 'white' ? 'black' : 'white';
            const normalizedScore = mover === 'black' ? -searchResult.score : searchResult.score;

            chess.addMove({
                san: local.san,
                uci: searchResult.bestmove,
                fen: local.fen,
                score: normalizedScore,
                fromBook: searchResult.from_book,
                mover,
            });
            chess.setEval(normalizedScore);

            if (engineResult.status !== 'active') {
                chess.setStatus(engineResult.status, deriveWinner(engineResult.status, mover));
                chess.showModal();
            }
        } finally {
            if (gameKeyRef.current === keyAtStart) chess.setThinking(false);
        }
    }, [engine, chess]);

    const handleStartGame = useCallback(async (playerColor: PlayerColor, difficulty: DifficultyLevel) => {
        await engine.newGame();
        chess.startNewGame(playerColor, difficulty);
        setShowNewGameDialog(false);

        if (playerColor === 'black') {
            const keyAtStart = chess.state.gameKey + 1; // reducer bumps it synchronously; ref effect hasn't run yet
            runEngineTurn(difficulty, keyAtStart);
        }
    }, [engine, chess, runEngineTurn]);

    const executeMove = useCallback((source: string, target: string, piece: string): boolean => {
        const isPromotion = piece[1] === 'P' && (target[1] === '8' || target[1] === '1');
        const uci = source + target + (isPromotion ? 'q' : '');

        const local = chess.applyLocalMove(uci);
        if (!local) return false;

        const keyAtStart = gameKeyRef.current;
        const mover = chess.state.playerColor;

        chess.addMove({ san: local.san, uci, fen: local.fen, score: chess.state.currentEval, fromBook: false, mover });

        engine.makeMove(uci).then((result) => {
            if (gameKeyRef.current !== keyAtStart) return;

            if (result.status !== 'active') {
                chess.setStatus(result.status, deriveWinner(result.status, mover));
                chess.showModal();
                return;
            }
            runEngineTurn(chess.state.difficulty, keyAtStart);
        });

        return true;
    }, [chess, engine, runEngineTurn]);

    const interactive = chess.state.status === 'active' && !chess.state.isThinking && !chess.isBrowsingHistory;

    const onPieceDrop = useCallback((source: string, target: string, piece: string): boolean => {
        if (!interactive) return false;
        return executeMove(source, target, piece);
    }, [interactive, executeMove]);

    const onSquareClick = useCallback((square: string, piece: string | undefined) => {
        if (!interactive) return;

        const isOwnPiece = !!piece && piece[0] === chess.state.playerColor[0];

        const selectSquare = (sq: string, pc: string) => {
            setSelectedSquare(sq);
            setSelectedPiece(pc);
            setLegalDestinations([]);
            const key = ++selectionKeyRef.current;
            engine.getLegalMovesForSquare(sq).then((uciMoves) => {
                if (selectionKeyRef.current !== key) return;
                setLegalDestinations([...new Set(uciMoves.map((uci) => uci.slice(2, 4)))]);
            });
        };

        if (!selectedSquare) {
            if (isOwnPiece) selectSquare(square, piece!);
            return;
        }

        if (square === selectedSquare) {
            clearSelection();
            return;
        }

        if (legalDestinations.includes(square)) {
            executeMove(selectedSquare, square, selectedPiece!);
            clearSelection();
            return;
        }

        if (isOwnPiece) {
            selectSquare(square, piece!);
            return;
        }

        clearSelection();
    }, [interactive, chess.state.playerColor, selectedSquare, selectedPiece, legalDestinations, executeMove, engine, clearSelection]);

    useEffect(() => {
        if (!interactive) clearSelection();
    }, [interactive, clearSelection]);

    const handleResign = useCallback(() => {
        if (chess.state.status !== 'active') return;
        const winner: PlayerColor = chess.state.playerColor === 'white' ? 'black' : 'white';
        chess.setStatus('resigned', winner);
        chess.showModal();
    }, [chess]);

    useEffect(() => {
        const onKeyDown = (e: KeyboardEvent) => {
            if (e.key === 'ArrowLeft') chess.navigateTo(Math.max(-1, chess.state.currentMoveIndex - 1));
            if (e.key === 'ArrowRight') chess.navigateTo(Math.min(chess.state.moves.length - 1, chess.state.currentMoveIndex + 1));
            if (e.key === 'Home') chess.navigateTo(-1);
            if (e.key === 'End') chess.navigateTo(chess.state.moves.length - 1);
        };
        document.addEventListener('keydown', onKeyDown);
        return () => document.removeEventListener('keydown', onKeyDown);
    }, [chess]);

    if (isLoading) return <LoadingScreen />;
    if (error) return <div className="p-6 text-destructive">Error: {error}</div>;

    const theme = resolvedTheme === 'light' ? 'light' : 'dark';
    const accentColors = ACCENT_COLORS[theme];
    const squareStyles: Record<string, React.CSSProperties> = {};
    if (selectedSquare && interactive) {
        squareStyles[selectedSquare] = { backgroundColor: accentColors.selected };
        for (const dest of legalDestinations) {
            squareStyles[dest] = chess.hasPieceAtSquare(dest)
                ? { outline: `3px solid ${accentColors.legalMove}`, outlineOffset: '-3px', borderRadius: '2px' }
                : { background: `radial-gradient(circle, ${accentColors.legalMove} 28%, transparent 28%)` };
        }
    }

    return (
        <div ref={containerRef} className="flex min-h-screen items-center justify-center p-6">
            <div className="flex gap-6">
                <div className="hidden md:block">
                    <EvalBar
                        score={chess.state.currentEval}
                        isThinking={chess.state.isThinking}
                        height={boardSize}
                    />
                </div>
                <div className="flex flex-col items-center gap-3">
                    <Board
                        fen={chess.state.displayFen}
                        orientation={chess.state.boardOrientation}
                        onPieceDrop={onPieceDrop}
                        onSquareClick={onSquareClick}
                        customSquareStyles={squareStyles}
                        interactive={interactive}
                        theme={theme}
                        size={boardSize}
                    />
                    <GameControls
                        onNewGame={() => setShowNewGameDialog(true)}
                        onFlipBoard={chess.flipBoard}
                        onResign={handleResign}
                        resignDisabled={chess.state.status !== 'active'}
                    />
                    <p className="text-center text-xs text-muted-foreground md:hidden">
                        Eval bar and move history available on desktop.
                    </p>
                </div>
                <div className="hidden w-60 overflow-y-auto rounded-lg border md:block" style={{ height: boardSize }}>
                    <MoveHistory moves={chess.state.moves} currentMoveIndex={chess.state.currentMoveIndex} onNavigate={chess.navigateTo} />
                </div>
            </div>

            <NewGameDialog open={showNewGameDialog} onStart={handleStartGame} />
            <GameOverModal
                open={chess.state.showModal && chess.state.status !== 'active'}
                status={chess.state.status}
                winner={chess.state.winner}
                onNewGame={() => setShowNewGameDialog(true)}
                onClose={chess.hideModal}
            />
        </div>
    );
}