export const BOARD_COLORS = {
    dark: {
        light: '#e8dcc8',   // reuse your --secondary light tone
        dark: '#a0826d',    // reuse your --accent
    },
    light: {
        light: '#f5f1e8',   // your --card
        dark: '#d1c4ad',    // your --divider
    },
} as const

export const ACCENT_COLORS = {
    dark: { lastMove: '', check: '', good: '', blunder: '' },
    light: { lastMove: '', check: '', good: '', blunder: '' },
} as const