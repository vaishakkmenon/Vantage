'use client';

import { useState } from 'react';

const DIFFICULTY_CONFIGS = {
    beginner: { label: 'Beginner', depth: 4, time: 'Instant' },
    intermediate: { label: 'Intermediate', depth: 10, time: 'Fast' },
    advanced: { label: 'Advanced', depth: 16, time: 'Think' },
    expert: { label: 'Expert', depth: 64, time: 'Slow' },
};

export default function NewGameDialog({ isOpen, onClose, onStart }) {
    const [selectedDifficulty, setSelectedDifficulty] = useState('intermediate');
    const [selectedColor, setSelectedColor] = useState('white');

    const handleStart = () => {
        onStart(selectedDifficulty, selectedColor);
        onClose();
    };

    return (
        <div style={{
            position: 'fixed', inset: 0,
            background: 'rgba(0, 0, 0, 0.75)',
            backdropFilter: 'blur(8px)',
            WebkitBackdropFilter: 'blur(8px)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            zIndex: 2000,
            animation: 'fadeIn 0.2s ease-out',
        }}>
            <div style={{
                background: '#1e1e1e',
                border: '1px solid #333',
                padding: '32px',
                borderRadius: '16px',
                width: '100%',
                maxWidth: '400px',
                boxShadow: '0 20px 50px rgba(0, 0, 0, 0.5)',
            }}>
                <h2 style={{ margin: '0 0 24px 0', color: 'white', fontSize: '24px', textAlign: 'center' }}>
                    New Game
                </h2>
            </div>
        </div >

    );
}