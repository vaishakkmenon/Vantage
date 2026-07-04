'use client';

function ControlButton({ icon, label, onClick, disabled, danger = false }) {
    return (
        <button
            onClick={onClick}
            disabled={disabled}
            title={label}
            style={{
                background: 'transparent',
                border: '1px solid rgba(255, 255, 255, 0.1)',
                color: disabled ? 'rgba(255, 255, 255, 0.2)' : (danger ? '#ff5252' : '#fff'),
                borderRadius: '8px',
                padding: '8px 16px',
                cursor: disabled ? 'not-allowed' : 'pointer',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: '8px',
                fontSize: '14px',
                fontWeight: '500',
                transition: 'all 0.2s ease',
                minWidth: '40px',
            }}
            onMouseEnter={(e) => {
                if (!disabled) {
                    e.target.style.background = danger ? 'rgba(255, 82, 82, 0.1)' : 'rgba(255, 255, 255, 0.1)';
                    e.target.style.borderColor = danger ? 'rgba(255, 82, 82, 0.3)' : 'rgba(255, 255, 255, 0.3)';
                }
            }}
            onMouseLeave={(e) => {
                e.target.style.background = 'transparent';
                e.target.style.borderColor = 'rgba(255, 255, 255, 0.1)';
            }}
        >
            <span style={{ fontSize: '18px', lineHeight: 1 }}>{icon}</span>
            <span style={{ display: 'none' }}>{label}</span>
        </button>
    );
}

export default function GameControls({ onNewGame, onFlipBoard, onResign }) {
    return (
        <div style={{
            display: 'flex',
            gap: '8px',
            marginTop: '16px',
            justifyContent: 'center',
            padding: '12px',
            background: '#1e1e1e',
            borderRadius: '8px',
            border: '1px solid #333',
        }}>
            <ControlButton
                icon="✚"
                label="New Game"
                onClick={onNewGame}
            />
            <ControlButton
                icon="↻"
                label="Flip Board"
                onClick={onFlipBoard}
            />
            <ControlButton
                icon="⚑"
                label="Resign"
                onClick={onResign}
                danger={true}
            />
        </div>
    );
}