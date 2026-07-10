import { GameStatus, GameWinner } from '../types'

export const getStatusText = (status: GameStatus, winner?: GameWinner): string => {
    switch (status) {
        case 'checkmate':
            return winner === 'white' ? 'White wins by checkmate' : winner === 'black' ? 'Black wins by checkmate' : 'Checkmate'
        case 'stalemate':
            return 'Draw by stalemate'
        case 'resigned':
            return winner === 'white' ? 'White wins by resignation' : 'Black wins by resignation'
        case 'draw_threefold':
            return 'Draw by repetition'
        case 'draw_50move':
            return 'Draw by 50-move rule'
        case 'draw_dead':
            return 'Draw by insufficient material'
        case 'draw_fivefold':
            return 'Draw by fivefold repetition'
        case 'draw_75move':
            return 'Draw by 75-move rule'
        default:
            return ''
    }
}