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
    dark: {
        lastMove: 'rgba(245, 158, 11, 0.35)',  // soft amber glow
        check: 'rgba(239, 68, 68, 0.45)',   // muted red/coral, pulsing
        good: 'rgba(34, 197, 94, 0.35)',   // dim green
        blunder: 'rgba(239, 68, 68, 0.3)',    // dim red
        selected: 'rgba(125, 179, 216, 0.45)',  // light blue, tap-to-move selection
        legalMove: 'rgba(125, 179, 216, 0.65)',  // light blue, tap-to-move destination markers
    },
    light: {
        lastMove: 'rgba(160, 130, 109, 0.35)', // warm clay glow, ties to --accent
        check: 'rgba(220, 38, 38, 0.4)',
        good: 'rgba(22, 163, 74, 0.3)',
        blunder: 'rgba(220, 38, 38, 0.28)',
        selected: 'rgba(125, 179, 216, 0.40)',  // light blue
        legalMove: 'rgba(125, 179, 216, 0.60)',
    },
} as const