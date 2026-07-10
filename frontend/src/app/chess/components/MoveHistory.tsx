'use client';

import { MoveRecord } from '../types';
import { cn } from '@/lib/utils';

type Props = {
    moves: MoveRecord[];
    currentMoveIndex: number;
    onNavigate: (index: number) => void;
};

export function MoveHistory({ moves, currentMoveIndex, onNavigate }: Props) {
    const pairs: { num: number; white?: MoveRecord; black?: MoveRecord; whiteIdx?: number; blackIdx?: number }[] = [];
    for (let i = 0; i < moves.length; i += 2) {
        pairs.push({
            num: i / 2 + 1,
            white: moves[i],
            whiteIdx: i,
            black: moves[i + 1],
            blackIdx: i + 1,
        });
    }

    return (
        <div className="flex flex-col overflow-y-auto text-sm">
            {pairs.map((p) => (
                <div key={p.num} className="flex items-center gap-2 px-3 py-1">
                    <span className="w-6 text-muted-foreground">{p.num}.</span>
                    <MoveCell san={p.white?.san} active={p.whiteIdx === currentMoveIndex} onClick={() => p.whiteIdx !== undefined && onNavigate(p.whiteIdx)} />
                    <MoveCell san={p.black?.san} active={p.blackIdx === currentMoveIndex} onClick={() => p.blackIdx !== undefined && onNavigate(p.blackIdx)} />
                </div>
            ))}
        </div>
    );
}

function MoveCell({ san, active, onClick }: { san?: string; active: boolean; onClick: () => void }) {
    if (!san) return <span className="w-14" />;
    return (
        <button
            onClick={onClick}
            className={cn(
                'w-14 rounded px-1 text-left hover:bg-accent',
                active && 'bg-accent font-semibold text-accent-foreground'
            )}
        >
            {san}
        </button>
    );
}