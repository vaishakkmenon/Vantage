import { SearchOptions, SearchResult, MakeMoveResult, GameStatus, PlayerColor } from '../types'

export interface EngineAdapter {
    newGame: () => Promise<void>
    setPosition: (fen: string) => Promise<void>
    setPositionStartpos: (moves?: string) => Promise<void>
    makeMove: (uci: string) => Promise<MakeMoveResult>
    isMoveLegal: (uci: string) => Promise<boolean>
    search: (options?: SearchOptions) => Promise<SearchResult>
    getLegalMoves: () => Promise<string[]>
    getLegalMovesForSquare: (square: string) => Promise<string[]>
    getFen: () => Promise<string>
    getGameStatus: () => Promise<GameStatus>
    getSideToMove: () => Promise<PlayerColor>
}