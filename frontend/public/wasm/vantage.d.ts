/* tslint:disable */
/* eslint-disable */

export class VantageEngine {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Apply a single UCI move (e.g., "e2e4", "e7e8q"). Returns true if legal.
     */
    apply_move(uci_move: string): boolean;
    /**
     * Get the current board position as a FEN string.
     */
    get_fen(): string;
    /**
     * Get current game status: "active", "checkmate", "stalemate", "draw_*"
     */
    get_game_status(): string;
    /**
     * Get all legal moves as a JSON array: ["e2e4", "d2d4", ...]
     */
    get_legal_moves(): string;
    /**
     * Get legal moves for a specific square (e.g., "e2")
     * Returns JSON array: ["e2e4", "e2e3"] or empty array if no piece/illegal square
     */
    get_legal_moves_for_square(square: string): string;
    /**
     * Search to a fixed depth. Returns JSON:
     * { "bestmove": "e2e4", "score": 35, "from_book": false }
     */
    go_depth(depth: number): string;
    /**
     * Search for a fixed time in milliseconds. Returns same JSON as go_depth.
     */
    go_movetime(ms: number): string;
    /**
     * Check if a specific UCI move is legal. Returns true/false.
     */
    is_move_legal(uci_move: string): boolean;
    /**
     * Make a move and return success status.
     * Returns JSON: {"valid": true/false, "fen": "...", "status": "active|checkmate|stalemate|draw"}
     */
    make_move(uci_move: string): string;
    /**
     * Initialize the engine. This is expensive due to magic table generation.
     * Call once.
     */
    constructor();
    /**
     * Reset to a new game (starting position, clear TT)
     */
    new_game(): void;
    /**
     * Set position from FEN string. Returns true on success.
     */
    set_position_fen(fen: string): boolean;
    /**
     * Set starting position and apply a sequence of UCI moves.
     * `moves_str` is space-separated: "e2e4 e7e5 g1f3"
     */
    set_position_startpos(moves_str: string): void;
    /**
     * Get whose turn it is: "white" or "black"
     */
    side_to_move(): string;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_vantageengine_free: (a: number, b: number) => void;
    readonly vantageengine_apply_move: (a: number, b: number, c: number) => number;
    readonly vantageengine_get_fen: (a: number) => [number, number];
    readonly vantageengine_get_game_status: (a: number) => [number, number];
    readonly vantageengine_get_legal_moves: (a: number) => [number, number];
    readonly vantageengine_get_legal_moves_for_square: (a: number, b: number, c: number) => [number, number];
    readonly vantageengine_go_depth: (a: number, b: number) => [number, number];
    readonly vantageengine_go_movetime: (a: number, b: number) => [number, number];
    readonly vantageengine_is_move_legal: (a: number, b: number, c: number) => number;
    readonly vantageengine_make_move: (a: number, b: number, c: number) => [number, number];
    readonly vantageengine_new: () => number;
    readonly vantageengine_new_game: (a: number) => void;
    readonly vantageengine_set_position_fen: (a: number, b: number, c: number) => number;
    readonly vantageengine_set_position_startpos: (a: number, b: number, c: number) => void;
    readonly vantageengine_side_to_move: (a: number) => [number, number];
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
