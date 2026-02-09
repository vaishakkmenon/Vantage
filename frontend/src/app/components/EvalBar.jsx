'use client';

export default function EvalBar({ score = 0, isThinking = false, isMate = false }) {
    const safeScore = Number(score) || 0;

    // 1. Calculate Percentages
    let percent;
    let displayText;

    if (isMate) {
        percent = safeScore > 0 ? 100 : 0;
        displayText = `M${Math.abs(safeScore)}`;
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
            // REMOVED minHeight and height. 
            // The parent flex container in ChessBoard.jsx (alignItems: stretch) 
            // will force this to match the board's height exactly.
            borderRadius: '8px', // Matched to your Board's border radius
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