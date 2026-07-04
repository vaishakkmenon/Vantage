'use client';

export default function EvalBar({ score = 0, isThinking = false, isMate = false }) {
    const safeScore = Number(score) || 0;

    // Auto-detect mate if score is huge (standard engines use +/- 30000 range for mate)
    // Threshold: 10000 cp (100 pawns)
    const isMateScore = Math.abs(safeScore) > 10000;

    let percent;
    let displayText;

    if (isMate || isMateScore) {
        percent = safeScore > 0 ? 100 : 0;

        const MATE_SCORE_CONSTANT = 31000;
        const pliesToMate = MATE_SCORE_CONSTANT - Math.abs(safeScore);
        const movesToMate = Math.ceil(pliesToMate / 2);

        const dist = (movesToMate > 0 && movesToMate < 100) ? movesToMate : "";
        displayText = `M${dist}`;
    } else {
        const clamped = Math.max(-1000, Math.min(1000, safeScore));
        percent = 50 + (clamped / 1000) * 50;

        const pawnScore = (safeScore / 100).toFixed(1);
        if (Math.abs(safeScore) < 5) {
            displayText = "0.0";
        } else {
            displayText = safeScore > 0 ? `+${pawnScore}` : pawnScore;
        }
    }

    const whiteFlex = Math.max(0.1, percent);
    const blackFlex = Math.max(0.1, 100 - percent);
    const isWhiteWinning = percent >= 50;

    return (
        <div style={{
            width: '32px',
            borderRadius: '8px',
            overflow: 'hidden',
            boxShadow: '0 4px 12px rgba(0,0,0,0.4)',
            display: 'flex',
            flexDirection: 'column',
            backgroundColor: '#2a2a2a',
        }}>
            {/* BLACK BAR (Top) */}
            <div style={{
                flex: blackFlex,
                backgroundColor: '#2a2a2a',
                transition: 'flex 0.5s cubic-bezier(0.4, 0, 0.2, 1)',
                position: 'relative',
                display: 'flex',
                flexDirection: 'column',
                justifyContent: 'flex-end',
                overflow: 'hidden',
            }}>
                {!isWhiteWinning && (
                    <span style={{
                        width: '100%',
                        textAlign: 'center',
                        fontSize: '11px',
                        fontWeight: '700',
                        color: '#a0a0a0',
                        paddingBottom: '4px',
                        display: 'block',
                    }}>
                        {displayText}
                    </span>
                )}
            </div>

            {/* WHITE BAR (Bottom) */}
            <div style={{
                flex: whiteFlex,
                backgroundColor: 'rgba(255, 255, 255, 0.85)',
                transition: 'flex 0.5s cubic-bezier(0.4, 0, 0.2, 1)',
                animation: isThinking ? 'pulse 1.5s infinite ease-in-out' : 'none',
                position: 'relative',
                display: 'flex',
                flexDirection: 'column',
                justifyContent: 'flex-start',
                overflow: 'hidden',
            }}>
                {isWhiteWinning && (
                    <span style={{
                        width: '100%',
                        textAlign: 'center',
                        fontSize: '11px',
                        fontWeight: '700',
                        color: '#2a2a2a',
                        paddingTop: '4px',
                        display: 'block',
                    }}>
                        {displayText}
                    </span>
                )}
            </div>

            <style jsx>{`
                @keyframes pulse {
                    0% { opacity: 0.85; }
                    50% { opacity: 0.6; }
                    100% { opacity: 0.85; }
                }
            `}</style>
        </div>
    );
}