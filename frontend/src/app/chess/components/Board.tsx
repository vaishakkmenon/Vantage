'use client';

import { Chessboard } from 'react-chessboard';
import { BOARD_COLORS } from '../lib/theme';
import { PlayerColor } from '../types';

type Props = {
    fen: string;
    orientation: PlayerColor;
    onPieceDrop: (source: string, target: string, piece: string) => boolean;
    interactive: boolean;
    theme: 'dark' | 'light';
    size: number;
};

export function Board({ fen, orientation, onPieceDrop, interactive, theme, size }: Props) {
    const colors = BOARD_COLORS[theme];
    return (
        <Chessboard
            position={fen}
            onPieceDrop={interactive ? onPieceDrop : () => false}
            boardOrientation={orientation}
            arePiecesDraggable={interactive}
            boardWidth={size}
            animationDuration={150}
            customDarkSquareStyle={{ backgroundColor: colors.dark }}
            customLightSquareStyle={{ backgroundColor: colors.light }}
            customBoardStyle={{ borderRadius: '8px', boxShadow: '0 4px 20px rgba(0,0,0,0.3)' }}
        />
    );
}