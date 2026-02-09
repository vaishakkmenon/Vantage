'use client';

import { useEffect, useRef } from 'react';

// Simple Book Icon Component
function BookIcon() {
    return (
        <svg
            viewBox="0 0 24 24"
            width="12"
            height="12"
            fill="currentColor"
            style={{ opacity: 0.7 }}
        >
            <path d="M4 6H2v14c0 1.1.9 2 2 2h14v-2H4V6zm16-4H8c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zm-1 9H9V9h10v2zm-4 4H9v-2h6v2zm4-8H9V5h10v2z" />
        </svg>
    );
}

export default function MoveHistory({ moves = [], currentMoveIndex, onMoveClick }) {
    const scrollRef = useRef(null);

    // Auto-scroll to bottom when new moves are added
    useEffect(() => {
        if (scrollRef.current) {
            scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
        }
    }, [moves]);

    // Group moves into pairs: [[white, black], [white, black], ...]
    const movePairs = [];
    for (let i = 0; i < moves.length; i += 2) {
        movePairs.push({
            white: moves[i],
            black: moves[i + 1] || null,
            moveNumber: Math.floor(i / 2) + 1
        });
    }

    if (moves.length === 0) {
        return (
            <div style={{
                color: 'rgba(255, 255, 255, 0.3)',
                fontSize: '14px',
                fontStyle: 'italic',
                padding: '20px',
                textAlign: 'center',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                height: '100%',
            }}>
                No moves yet
            </div>
        );
    }

    return (
        <div
            ref={scrollRef}
            style={{
                height: '100%',
                overflowY: 'auto',
                padding: '0 8px',
                fontFamily: '"Roboto Mono", monospace',
                fontSize: '13px',
                scrollbarWidth: 'thin',
                scrollbarColor: '#4a4a4a #1a1a1a',
            }}
        >
            <table style={{ width: '100%', borderCollapse: 'collapse' }}>
                <tbody>
                    {movePairs.map((pair, index) => {
                        const whiteIndex = index * 2;
                        const blackIndex = index * 2 + 1;

                        const isWhiteActive = currentMoveIndex === whiteIndex;
                        const isBlackActive = currentMoveIndex === blackIndex;

                        return (
                            <tr key={index} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                                {/* Move Number */}
                                <td style={{
                                    width: '30px',
                                    color: '#666',
                                    padding: '4px 0',
                                    verticalAlign: 'middle'
                                }}>
                                    {pair.moveNumber}.
                                </td>

                                {/* White Move */}
                                <td
                                    onClick={() => onMoveClick && onMoveClick(whiteIndex)}
                                    style={{
                                        padding: '4px 8px',
                                        cursor: 'pointer',
                                        borderRadius: '4px',
                                        backgroundColor: isWhiteActive ? 'rgba(255, 255, 255, 0.15)' : 'transparent',
                                        color: isWhiteActive ? '#fff' : 'rgba(255, 255, 255, 0.7)',
                                        transition: 'background-color 0.2s',
                                        display: 'table-cell',
                                        verticalAlign: 'middle',
                                    }}
                                >
                                    <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                                        {pair.white.san}
                                        {pair.white.from_book && (
                                            <span style={{ color: '#4caf50', display: 'flex', alignItems: 'center' }} title="Book Move">
                                                <BookIcon />
                                            </span>
                                        )}
                                    </div>
                                </td>

                                {/* Black Move */}
                                <td
                                    onClick={() => pair.black && onMoveClick && onMoveClick(blackIndex)}
                                    style={{
                                        padding: '4px 8px',
                                        cursor: 'pointer',
                                        borderRadius: '4px',
                                        backgroundColor: isBlackActive ? 'rgba(255, 255, 255, 0.15)' : 'transparent',
                                        color: isBlackActive ? '#fff' : 'rgba(255, 255, 255, 0.7)',
                                        transition: 'background-color 0.2s',
                                        display: 'table-cell',
                                        verticalAlign: 'middle',
                                    }}
                                >
                                    {pair.black ? (
                                        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                                            {pair.black.san}
                                            {pair.black.from_book && (
                                                <span style={{ color: '#4caf50', display: 'flex', alignItems: 'center' }} title="Book Move">
                                                    <BookIcon />
                                                </span>
                                            )}
                                        </div>
                                    ) : ''}
                                </td>
                            </tr>
                        );
                    })}
                </tbody>
            </table>

            <style jsx>{`
                div::-webkit-scrollbar {
                    width: 6px;
                }
                div::-webkit-scrollbar-track {
                    background: #1a1a1a; 
                }
                div::-webkit-scrollbar-thumb {
                    background: #4a4a4a; 
                    border-radius: 3px;
                }
                div::-webkit-scrollbar-thumb:hover {
                    background: #555; 
                }
            `}</style>
        </div>
    );
}