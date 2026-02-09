'use client';

export default function GameOverModal({ status, onNewGame }) {
    const isCheckmate = status === 'checkmate';

    const messages = {
        checkmate: 'Checkmate',
        stalemate: 'Stalemate',
        draw_threefold: 'Threefold Repetition',
        draw_50move: '50 Move Rule',
        draw_fivefold: 'Fivefold Repetition',
        draw_75move: '75 Move Rule',
        draw_dead: 'Insufficient Material',
    };

    const subtitle = isCheckmate ? 'The engine wins.' : 'The game is a draw.';

    return (
        <div style={{
            position: 'fixed', inset: 0,
            background: 'rgba(0, 0, 0, 0.75)',
            backdropFilter: 'blur(8px)',
            WebkitBackdropFilter: 'blur(8px)',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            zIndex: 1000,
            animation: 'fadeIn 0.3s ease-out',
        }}>
            <div style={{
                background: 'linear-gradient(145deg, #1a1a1a 0%, #111111 100%)',
                border: '1px solid rgba(255, 255, 255, 0.08)',
                padding: '48px 40px',
                borderRadius: '16px',
                textAlign: 'center',
                minWidth: '320px',
                maxWidth: '400px',
                boxShadow: '0 25px 60px rgba(0, 0, 0, 0.6), 0 0 40px rgba(0, 0, 0, 0.3)',
                animation: 'slideUp 0.4s cubic-bezier(0.16, 1, 0.3, 1)',
            }}>
                {/* Icon */}
                <div style={{
                    fontSize: '48px',
                    marginBottom: '20px',
                    filter: 'drop-shadow(0 4px 8px rgba(0,0,0,0.3))',
                }}>
                    {isCheckmate ? '♚' : '½'}
                </div>

                {/* Title */}
                <h2 style={{
                    fontSize: '28px',
                    fontWeight: 700,
                    color: '#ffffff',
                    marginBottom: '8px',
                    letterSpacing: '-0.5px',
                }}>
                    {messages[status] || 'Game Over'}
                </h2>

                {/* Subtitle */}
                <p style={{
                    fontSize: '15px',
                    color: 'rgba(255, 255, 255, 0.5)',
                    marginBottom: '32px',
                    fontWeight: 400,
                }}>
                    {subtitle}
                </p>

                <div style={{
                    height: '1px',
                    background: 'linear-gradient(90deg, transparent, rgba(255,255,255,0.1), transparent)',
                    marginBottom: '28px',
                }} />

                <button
                    onClick={onNewGame}
                    style={{
                        padding: '12px 36px',
                        fontSize: '15px',
                        fontWeight: 600,
                        background: 'rgba(255, 255, 255, 0.9)',
                        color: '#0a0a0a',
                        border: 'none',
                        borderRadius: '10px',
                        cursor: 'pointer',
                        transition: 'all 0.2s ease',
                        letterSpacing: '0.3px',
                    }}
                    onMouseEnter={(e) => {
                        e.target.style.background = '#ffffff';
                        e.target.style.transform = 'translateY(-1px)';
                        e.target.style.boxShadow = '0 4px 12px rgba(255, 255, 255, 0.15)';
                    }}
                    onMouseLeave={(e) => {
                        e.target.style.background = 'rgba(255, 255, 255, 0.9)';
                        e.target.style.transform = 'translateY(0)';
                        e.target.style.boxShadow = 'none';
                    }}
                >
                    New Game
                </button>
            </div>
            <style jsx>{`
                @keyframes fadeIn {
                    from { opacity: 0; }
                    to { opacity: 1; }
                }
                @keyframes slideUp {
                    from { opacity: 0; transform: translateY(20px) scale(0.96); }
                    to { opacity: 1; transform: translateY(0) scale(1); }
                }
            `}</style>
        </div>
    );
}