'use client';

import { useState } from 'react';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { Button } from '@/components/ui/button';
import { PlayerColor, DifficultyLevel } from '../types';
import { DIFFICULTY_CONFIGS } from '../lib/difficulty';

type Props = {
    open: boolean;
    onStart: (playerColor: PlayerColor, difficulty: DifficultyLevel) => void;
};

export function NewGameDialog({ open, onStart }: Props) {
    const [playerColor, setPlayerColor] = useState<PlayerColor>('white');
    const [difficulty, setDifficulty] = useState<DifficultyLevel>('intermediate');

    return (
        <Dialog open={open}>
            <DialogContent showCloseButton={false} onInteractOutside={(e) => e.preventDefault()}>
                <DialogHeader>
                    <DialogTitle>New Game</DialogTitle>
                </DialogHeader>

                <div className="space-y-4">
                    <div>
                        <Label className="mb-2 block">Play as</Label>
                        <RadioGroup value={playerColor} onValueChange={(v) => setPlayerColor(v as PlayerColor)} className="flex gap-4">
                            <div className="flex items-center gap-2">
                                <RadioGroupItem value="white" id="color-white" />
                                <Label htmlFor="color-white">White</Label>
                            </div>
                            <div className="flex items-center gap-2">
                                <RadioGroupItem value="black" id="color-black" />
                                <Label htmlFor="color-black">Black</Label>
                            </div>
                        </RadioGroup>
                    </div>

                    <Separator />

                    <div>
                        <Label className="mb-2 block">Difficulty</Label>
                        <RadioGroup value={difficulty} onValueChange={(v) => setDifficulty(v as DifficultyLevel)} className="space-y-2">
                            {(Object.keys(DIFFICULTY_CONFIGS) as DifficultyLevel[]).map((level) => (
                                <div key={level} className="flex items-center gap-2">
                                    <RadioGroupItem value={level} id={`diff-${level}`} />
                                    <Label htmlFor={`diff-${level}`}>
                                        {DIFFICULTY_CONFIGS[level].label}
                                        <span className="ml-2 text-muted-foreground">{DIFFICULTY_CONFIGS[level].description}</span>
                                    </Label>
                                </div>
                            ))}
                        </RadioGroup>
                    </div>
                </div>

                <DialogFooter>
                    <Button onClick={() => onStart(playerColor, difficulty)}>Start Game</Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
}