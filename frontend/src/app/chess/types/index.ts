export type PlayerColor = 'white' | 'black'
export type DifficultyLevel = 'beginner' | 'intermediate' | 'advanced' | 'expert'
export type GameStatus = 'active' | 'checkmate' | 'stalemate' | 'draw_threefold' | 'draw_50move' | 'draw_fivefold' | 'draw_75move' | 'draw_dead' | 'resigned'
export type GameWinner = 'white' | 'black' | 'draw' | null
export type SearchOptions = {
    depth?: number;
    movetime?: number
}
export type SearchResult = {
    bestmove: string;
    score: number;
    from_book: boolean
}
export type MakeMoveResult = {
    valid: boolean;
    fen: string;
    status: GameStatus
}
export type MoveRecord = {
    san: string;
    uci: string;
    fen: string;
    score: number;
    fromBook: boolean;
    mover: PlayerColor
}
export type DifficultyConfig = {
    label: string;
    description: string;
    searchOptions: SearchOptions
}
export type ChessGameState = {
    playerColor: PlayerColor;
    difficulty: DifficultyLevel;
    currentMoveIndex: number;
    displayFen: string;
    liveFen: string;
    status: GameStatus;
    winner: GameWinner;
    isThinking: boolean;
    showModal: boolean;
    currentEval: number;
    gameKey: number,
    moves: MoveRecord[],
    boardOrientation: PlayerColor
}
