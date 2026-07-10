'use client';

import { Button } from '@/components/ui/button';

type Props = {
    onNewGame: () => void;
    onFlipBoard: () => void;
    onResign: () => void;
    resignDisabled: boolean;
};

export function GameControls({ onNewGame, onFlipBoard, onResign, resignDisabled }: Props) {
    return (
        <div className="flex w-full justify-center gap-2 py-3">
            <Button variant="outline" onClick={onNewGame} aria-label="Start new game">
                New Game
            </Button>
            <Button variant="outline" onClick={onFlipBoard} aria-label="Flip board orientation">
                Flip Board
            </Button>
            <Button variant="destructive" onClick={onResign} disabled={resignDisabled} aria-label="Resign game">
                Resign
            </Button>
        </div>
    );
}