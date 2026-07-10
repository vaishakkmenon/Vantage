'use client';

import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { GameStatus, GameWinner } from '../types';
import { getStatusText } from '../lib/gameStatus';

type Props = {
    open: boolean;
    status: GameStatus;
    winner: GameWinner;
    onNewGame: () => void;
    onClose: () => void;
};

export function GameOverModal({ open, status, winner, onNewGame, onClose }: Props) {
    return (
        <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>{getStatusText(status, winner)}</DialogTitle>
                </DialogHeader>
                <DialogFooter>
                    <Button variant="outline" onClick={onClose}>
                        Review Board
                    </Button>
                    <Button onClick={onNewGame}>New Game</Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
}