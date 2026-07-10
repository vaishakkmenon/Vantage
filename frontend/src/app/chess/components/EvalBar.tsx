type Props = {
    score: number;       // centipawns, White's perspective (already normalized by orchestrator)
    isThinking: boolean;
    height: number;
};

const MATE_THRESHOLD = 30000;

function whitePercent(score: number): number {
    if (Math.abs(score) >= MATE_THRESHOLD) {
        return score > 0 ? 100 : 0;
    }
    const clamped = Math.max(-1000, Math.min(1000, score));
    return 50 + (clamped / 1000) * 50;
}

function evalLabel(score: number): string {
    if (Math.abs(score) >= MATE_THRESHOLD) {
        const movesToMate = MATE_SCORE_TO_MOVES(score);
        return `M${movesToMate}`;
    }
    return (score / 100).toFixed(1);
}

const MATE_SCORE = 31000;
function MATE_SCORE_TO_MOVES(score: number): number {
    const movesToMate = Math.floor((MATE_SCORE - Math.abs(score) + 1) / 2);
    return movesToMate * (score > 0 ? 1 : -1);
}

export function EvalBar({ score, isThinking, height }: Props) {
    const pct = whitePercent(score);
    return (
        <div className="relative w-6 overflow-hidden rounded-md bg-neutral-800" style={{ height }}>
            <div
                className="absolute bottom-0 w-full bg-neutral-100 transition-[height] duration-300"
                style={{ height: `${pct}%` }}
            />
            <div className="absolute inset-x-0 bottom-1 text-center text-[10px] font-medium text-neutral-500">
                {isThinking ? '…' : evalLabel(score)}
            </div>
        </div>
    );
}