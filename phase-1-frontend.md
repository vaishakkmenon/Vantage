# Phase 1: Vantage Chess Frontend

## Overview

Build the Next.js frontend for chess.vaishakmenon.com that lets visitors play against and analyze positions with the Vantage engine running as WASM in the browser.

**Prerequisites (all confirmed complete):**
- Phase 0 done: WASM compiles, 1.6MB binary, engine initializes in ~7s
- WASM API: `VantageEngine` with `new()`, `set_position_fen()`, `set_position_startpos()`, `apply_move()`, `go_depth()`, `go_movetime()`, `get_fen()`, `get_legal_moves()`, `side_to_move()`, `new_game()`
- Web Worker prototype: `test-worker.js` with message-based interface
- 329 native tests passing, no regressions

---

## Phase 1a: Core Board UI (2-3 days)

### Goal
A playable chess board where you can make moves against Vantage. No frills — just the core play loop working end-to-end.

### Task 1a.1: Next.js Project Scaffolding

**Create the frontend project inside the Vantage repo:**

```bash
cd Vantage
npx create-next-app@latest frontend --typescript --tailwind --eslint --app --src-dir
```

**Directory structure after scaffolding:**
```
Vantage/
├── backend/          # Existing Rust engine
├── frontend/         # New Next.js app
│   ├── public/
│   │   └── wasm/     # WASM output goes here
│   ├── src/
│   │   └── app/
│   │       ├── layout.tsx
│   │       ├── page.tsx        # Redirect or landing
│   │       └── chess/
│   │           └── page.tsx    # Chess UI
│   ├── next.config.ts
│   ├── tailwind.config.ts
│   ├── tsconfig.json
│   └── package.json
└── ...
```

**Copy WASM output to frontend:**
```bash
cd backend
wasm-pack build --target web --no-default-features --features psqt,load_magic
cp -r /workspace/backend/pkg /workspace/frontend/public/wasm
```

**Install chess dependencies:**
```bash
cd frontend
npm install react-chessboard
```

**Configure Next.js for WASM** (`next.config.ts`):
```typescript
const nextConfig = {
  webpack: (config) => {
    config.experiments = {
      ...config.experiments,
      asyncWebAssembly: true,
    };
    return config;
  },
  // Allow importing from public/wasm/
  async headers() {
    return [
      {
        source: '/wasm/:path*',
        headers: [
          { key: 'Cross-Origin-Opener-Policy', value: 'same-origin' },
          { key: 'Cross-Origin-Embedder-Policy', value: 'require-corp' },
        ],
      },
    ];
  },
};
export default nextConfig;
```

**Verify:** `npm run dev` starts without errors, visiting `/chess` shows a blank page.

---

### Task 1a.2: TypeScript Type Definitions

**New file:** `frontend/src/app/chess/types/index.ts`

```typescript
// Engine communication types
export interface SearchResult {
  bestmove: string;
  score: number;
  from_book: boolean;
}

export interface SearchInfo {
  depth: number;
  score: number;
  nodes: number;
  nps: number;
  time: number;
  pv: string[];
}

export interface EngineState {
  ready: boolean;
  searching: boolean;
  error: string | null;
}

// Game state types
export type GameMode = 'play' | 'analyze';
export type PlayerColor = 'white' | 'black';
export type GameResult = 'white' | 'black' | 'draw' | null;

export type DifficultyLevel = 'beginner' | 'intermediate' | 'advanced' | 'maximum';

export interface DifficultyConfig {
  label: string;
  depth: number;
  movetime: number; // ms
}

export const DIFFICULTY_CONFIGS: Record<DifficultyLevel, DifficultyConfig> = {
  beginner:     { label: 'Beginner',     depth: 4,  movetime: 500  },
  intermediate: { label: 'Intermediate', depth: 10, movetime: 2000 },
  advanced:     { label: 'Advanced',     depth: 16, movetime: 5000 },
  maximum:      { label: 'Maximum',      depth: 64, movetime: 30000 },
};

export type TimeControl = {
  initial: number;  // seconds
  increment: number; // seconds
} | null; // null = unlimited

export const TIME_CONTROLS: { label: string; value: TimeControl }[] = [
  { label: '1+0',       value: { initial: 60,  increment: 0  } },
  { label: '2+1',       value: { initial: 120, increment: 1  } },
  { label: '3+0',       value: { initial: 180, increment: 0  } },
  { label: '3+2',       value: { initial: 180, increment: 2  } },
  { label: '5+0',       value: { initial: 300, increment: 0  } },
  { label: '5+3',       value: { initial: 300, increment: 3  } },
  { label: '10+0',      value: { initial: 600, increment: 0  } },
  { label: '15+10',     value: { initial: 900, increment: 10 } },
  { label: 'Unlimited', value: null },
];

// Move history types
export interface MoveEntry {
  san: string;       // Standard algebraic: "Nf3"
  uci: string;       // UCI format: "g1f3"
  fen: string;       // Position after this move
  moveNumber: number;
  color: PlayerColor;
}

export interface GameState {
  mode: GameMode;
  playerColor: PlayerColor;
  difficulty: DifficultyLevel;
  timeControl: TimeControl;
  fen: string;
  moves: MoveEntry[];
  currentMoveIndex: number; // -1 = start position
  result: GameResult;
  isEngineThinking: boolean;
  boardFlipped: boolean;
}
```

---

### Task 1a.3: Engine Adapter Layer

This is the abstraction that lets us swap WASM for a VPS backend later.

**New file:** `frontend/src/app/chess/engine/EngineAdapter.ts`

```typescript
import { SearchResult, SearchInfo } from '../types';

export interface SearchOptions {
  depth?: number;
  movetime?: number; // milliseconds
}

export interface EngineAdapter {
  init(): Promise<void>;
  newGame(): void;
  setPosition(fen: string): void;
  setPositionStartpos(moves: string): void;
  applyMove(uci: string): boolean;
  search(options: SearchOptions): Promise<SearchResult>;
  getLegalMoves(): string[];
  getFen(): string;
  getSideToMove(): 'white' | 'black';
  onSearchInfo(callback: (info: SearchInfo) => void): void;
  quit(): void;
}
```

**New file:** `frontend/src/app/chess/engine/vantage.worker.ts`

Port the `test-worker.js` from Phase 0 to TypeScript with proper typing:

```typescript
// Web Worker that loads and runs the WASM engine
// Communication via postMessage with typed messages

interface WorkerRequest {
  id: number;
  type: 'init' | 'new_game' | 'set_position' | 'apply_move' | 'search' | 'get_legal_moves' | 'get_fen' | 'side_to_move';
  payload?: any;
}

interface WorkerResponse {
  id: number;
  type: string;
  payload: any;
  error?: string;
}

let engine: any = null;

self.onmessage = async (e: MessageEvent<WorkerRequest>) => {
  const { id, type, payload } = e.data;

  try {
    switch (type) {
      case 'init': {
        const wasm = await import('/wasm/vantage.js');
        await wasm.default();
        engine = new wasm.VantageEngine();
        respond(id, 'ready', {});
        break;
      }
      case 'new_game': {
        engine.new_game();
        respond(id, 'ok', {});
        break;
      }
      case 'set_position': {
        if (payload.fen) {
          engine.set_position_fen(payload.fen);
        } else {
          engine.set_position_startpos(payload.moves || '');
        }
        respond(id, 'ok', { fen: engine.get_fen() });
        break;
      }
      case 'apply_move': {
        const success = engine.apply_move(payload.move);
        respond(id, 'move_result', {
          success,
          fen: engine.get_fen(),
          side: engine.side_to_move(),
        });
        break;
      }
      case 'search': {
        let result: string;
        if (payload.depth) {
          result = engine.go_depth(payload.depth);
        } else if (payload.movetime) {
          result = engine.go_movetime(payload.movetime);
        } else {
          result = engine.go_depth(10); // default
        }
        respond(id, 'search_result', JSON.parse(result));
        break;
      }
      case 'get_legal_moves': {
        const moves = JSON.parse(engine.get_legal_moves());
        respond(id, 'legal_moves', { moves });
        break;
      }
      case 'get_fen': {
        respond(id, 'fen', { fen: engine.get_fen() });
        break;
      }
      case 'side_to_move': {
        respond(id, 'side', { side: engine.side_to_move() });
        break;
      }
    }
  } catch (err: any) {
    respond(id, 'error', {}, err.message || 'Unknown error');
  }
};

function respond(id: number, type: string, payload: any, error?: string) {
  self.postMessage({ id, type, payload, error } as WorkerResponse);
}
```

**New file:** `frontend/src/app/chess/engine/WasmEngine.ts`

```typescript
import { EngineAdapter, SearchOptions } from './EngineAdapter';
import { SearchResult, SearchInfo } from '../types';

export class WasmEngine implements EngineAdapter {
  private worker: Worker | null = null;
  private requestId = 0;
  private pendingRequests = new Map<number, { resolve: Function; reject: Function }>();
  private searchInfoCallback: ((info: SearchInfo) => void) | null = null;

  async init(): Promise<void> {
    this.worker = new Worker(
      new URL('./vantage.worker.ts', import.meta.url),
      { type: 'module' }
    );

    this.worker.onmessage = (e) => {
      const { id, type, payload, error } = e.data;
      const pending = this.pendingRequests.get(id);
      if (pending) {
        this.pendingRequests.delete(id);
        if (error) {
          pending.reject(new Error(error));
        } else {
          pending.resolve(payload);
        }
      }
    };

    await this.send('init');
  }

  newGame(): void { this.send('new_game'); }

  setPosition(fen: string): void {
    this.send('set_position', { fen });
  }

  setPositionStartpos(moves: string): void {
    this.send('set_position', { moves });
  }

  applyMove(uci: string): boolean {
    // Fire and forget — the game state hook handles sync
    this.send('apply_move', { move: uci });
    return true;
  }

  async search(options: SearchOptions): Promise<SearchResult> {
    const payload: any = {};
    if (options.depth) payload.depth = options.depth;
    if (options.movetime) payload.movetime = options.movetime;
    const result = await this.send('search', payload);
    return result as SearchResult;
  }

  async getLegalMoves(): Promise<string[]> {
    const result = await this.send('get_legal_moves');
    return result.moves;
  }

  async getFen(): Promise<string> {
    const result = await this.send('get_fen');
    return result.fen;
  }

  async getSideToMove(): Promise<'white' | 'black'> {
    const result = await this.send('side_to_move');
    return result.side;
  }

  onSearchInfo(callback: (info: SearchInfo) => void): void {
    this.searchInfoCallback = callback;
  }

  quit(): void {
    this.worker?.terminate();
    this.worker = null;
  }

  private send(type: string, payload?: any): Promise<any> {
    return new Promise((resolve, reject) => {
      const id = ++this.requestId;
      this.pendingRequests.set(id, { resolve, reject });
      this.worker?.postMessage({ id, type, payload });
    });
  }
}
```

---

### Task 1a.4: Game State Hook

**New file:** `frontend/src/app/chess/hooks/useGameState.ts`

Manages all game state with `useReducer`. Handles:
- Position tracking (FEN, move history)
- Turn management
- Game result detection (checkmate, stalemate, draw)
- Move navigation (click to jump to past positions)
- Board flip

Uses `chess.js` for move validation and SAN conversion on the UI side. The WASM engine is the source of truth for the position, but `chess.js` provides instant client-side validation so the UI feels responsive.

**Key state transitions:**
```
User clicks square → chess.js validates → Board updates instantly
                                        → WASM engine.apply_move()
                                        → Engine searches → bestmove received
                                        → chess.js applies engine move → Board updates
```

---

### Task 1a.5: useEngine Hook

**New file:** `frontend/src/app/chess/hooks/useEngine.ts`

Manages the engine lifecycle:
- Initialize WASM engine on mount
- Track loading state (show spinner during ~7s init)
- Send position/search commands
- Receive results and dispatch to game state
- Cleanup on unmount

---

### Task 1a.6: Board Component

**New file:** `frontend/src/app/chess/components/Board.tsx`

Wraps `react-chessboard` with:
- Dark theme board colors (muted dark/light squares matching portfolio)
- Legal move highlighting (dots on valid squares)
- Last move highlighting
- Check highlighting (red tint on king square)
- Drag-and-drop + click-click move input
- Board flip support
- Responsive sizing (fills available width, maintains square aspect ratio)

---

### Task 1a.7: ChessGame Orchestrator + Page

**New file:** `frontend/src/app/chess/components/ChessGame.tsx`

Top-level component that ties everything together:
- Initializes engine via `useEngine`
- Manages game state via `useGameState`
- Handles the play loop:
  1. User makes a move (via Board component)
  2. Move applied to chess.js and sent to engine
  3. Engine searches for response
  4. Engine's move applied to board
- Shows loading screen while WASM initializes

**New file:** `frontend/src/app/chess/page.tsx`

```typescript
export default function ChessPage() {
  return (
    <main className="min-h-screen bg-[your-portfolio-bg-color]">
      <ChessGame />
    </main>
  );
}
```

### Task 1a.8: Theme & Styling

**New file:** `frontend/src/app/chess/lib/theme.ts`

```typescript
export const boardTheme = {
  lightSquare: '#4a4a5a',   // Muted purple-grey
  darkSquare:  '#2d2d3d',   // Deep purple-grey
  lastMoveHighlight: 'rgba(155, 199, 0, 0.3)',
  legalMoveIndicator: 'rgba(255, 255, 255, 0.15)',
  checkHighlight: 'rgba(255, 0, 0, 0.4)',
};
```

Colors should match vaishakmenon.com's dark theme. Extract exact values from the portfolio site.

---

### Phase 1a Success Criteria

- [ ] `npm run dev` serves the chess page at `/chess`
- [ ] Board renders with dark theme
- [ ] User can make moves via drag-and-drop or click
- [ ] Legal moves highlighted on click
- [ ] Engine responds with a move after user moves
- [ ] Game detects checkmate/stalemate
- [ ] Loading spinner shown during WASM initialization (~7s)
- [ ] No console errors

### Phase 1a Estimated Time: 2-3 days

---

## Phase 1b: Game Features (2-3 days)

### Goal
Add the features that make it feel like a real chess interface — eval bar, move history, controls, difficulty settings, and clocks.

### Task 1b.1: Eval Bar

**New file:** `frontend/src/app/chess/components/EvalBar.tsx`

- Vertical bar to the left of the board
- White section grows from bottom for white advantage
- Height proportional to eval, scaled to ±10 range (eval/10 maps to 0-100%)
- Mate scores fill to 100%
- Numeric display: "+1.3" or "M5"
- Smooth CSS transitions on eval changes
- Matches board height exactly

### Task 1b.2: Move History Panel

**New file:** `frontend/src/app/chess/components/MoveHistory.tsx`

- Two-column layout: move number + white move + black move
- SAN notation (e.g., "1. e4 e5 2. Nf3 Nc6")
- Click any move to jump to that position
- Current move highlighted with accent color
- Auto-scrolls to latest move
- Empty state: "No moves yet"

### Task 1b.3: Engine Info Display

**New file:** `frontend/src/app/chess/components/EngineInfo.tsx`

Shows during/after engine search:
- Depth reached
- Nodes searched (formatted: "1.2M")
- Nodes per second (formatted: "850k")
- Search time
- Best move (if from book, show "Book" badge)

**Note:** This requires Phase 0's `search()` to emit info lines. If the current WASM API only returns the final result (bestmove + score), this panel shows limited info initially. Full info streaming requires the `on_info` callback refactor described in the main plan (Option B). For now, show what's available: score, whether it's a book move, and the bestmove.

### Task 1b.4: Game Controls

**New file:** `frontend/src/app/chess/components/GameControls.tsx`

Buttons:
- **New Game** — resets board, opens new game dialog
- **Flip Board** — toggles board orientation
- **Undo** — takes back last move pair (user + engine)
- **Resign** — ends the game

### Task 1b.5: Difficulty Selector

**New file:** `frontend/src/app/chess/components/DifficultySelector.tsx`

Shown in new game dialog or sidebar:
- Four levels: Beginner / Intermediate / Advanced / Maximum
- Each maps to depth + movetime limits (from DIFFICULTY_CONFIGS)
- Also select player color: White / Black / Random

### Task 1b.6: Time Controls & Clock

**New file:** `frontend/src/app/chess/components/ClockDisplay.tsx`
**New file:** `frontend/src/app/chess/hooks/useClock.ts`

- Two clocks (player + engine)
- Counts down on the active side's turn
- Increment added after each move
- Visual urgency: text turns red/orange when < 30s
- Flag detection (time runs out = loss)
- Clock pauses when game is over
- "Unlimited" option hides clocks

### Task 1b.7: Desktop Layout Assembly

Wire all components into the page layout:

```
┌────────────────────────────────────────────────────┐
│  Nav                                                │
├──────────────────────┬─────────────────────────────┤
│                      │  Engine Info                 │
│   ┌──┐               ├─────────────────────────────┤
│   │E │               │  Move History               │
│   │v │  Chessboard   │  (scrollable)               │
│   │a │               ├─────────────────────────────┤
│   │l │               │  Controls                   │
│   │  │               │  [New] [Flip] [Undo]        │
│   │B │               │  Difficulty: Advanced        │
│   │a │               │  Clock: 5:00 / 5:00         │
│   │r │               │                             │
│   └──┘               │                             │
├──────────────────────┴─────────────────────────────┤
│  Footer                                             │
└────────────────────────────────────────────────────┘
```

### Phase 1b Success Criteria

- [ ] Eval bar updates after each engine move
- [ ] Move history shows all moves in SAN, clickable to navigate
- [ ] Engine info displays score and book status
- [ ] New game, flip, undo, resign all work
- [ ] Difficulty selector changes engine depth/time limits
- [ ] Clocks count down and flag correctly
- [ ] Layout matches the wireframe

### Phase 1b Estimated Time: 2-3 days

---

## Phase 1c: Analysis & Polish (2-3 days)

### Goal
Add analysis mode, import/export features, responsive layout, and polish the experience.

### Task 1c.1: Analysis Mode Toggle

**New file:** `frontend/src/app/chess/components/ModeToggle.tsx`

- Toggle switch: "Play" ↔ "Analyze"
- In Analyze mode:
  - Both sides can move freely (no engine auto-response)
  - Engine runs continuously on current position
  - Forward/backward move navigation with arrow keys
  - No clock
- In Play mode:
  - Normal game flow (engine responds after user moves)

### Task 1c.2: FEN Import/Export

**New file:** `frontend/src/app/chess/components/FenInput.tsx`

- Text input field with "Load FEN" button
- Validates FEN before applying (chess.js `.validate_fen()`)
- "Copy FEN" button copies current position FEN to clipboard
- Shows in both Play (new game from position) and Analyze modes
- Error message for invalid FEN

### Task 1c.3: PGN Export

**New file:** `frontend/src/app/chess/components/PgnExport.tsx`

- "Copy PGN" button — copies to clipboard
- "Download PGN" button — downloads `.pgn` file
- PGN includes headers:
  ```
  [Event "Vantage Chess"]
  [Site "chess.vaishakmenon.com"]
  [Date "2026.02.08"]
  [White "Player"]
  [Black "Vantage (Advanced)"]
  [Result "1-0"]
  ```
- Generate PGN from chess.js game history

### Task 1c.4: PV Line Display (if search info available)

**New file:** `frontend/src/app/chess/components/PVLines.tsx`

- Shows top principal variation from engine
- Displayed in analysis mode below the engine info
- Click to preview on board (temporarily show the PV position)

**Note:** Full multi-PV requires the `on_info` callback refactor in the engine. For v1, show the bestmove line only. Mark multi-PV as a Phase 2 enhancement.

### Task 1c.5: Keyboard Navigation

Add keyboard shortcuts:
- `←` / `→` — navigate moves (backward/forward)
- `Home` / `End` — jump to start/end
- `F` — flip board
- `N` — new game

### Task 1c.6: Loading State

- Full-screen loading overlay during WASM initialization (~7s)
- Animated chess piece or engine logo
- Progress indicator: "Loading engine..." → "Initializing..." → "Ready"
- Fade transition when ready

### Task 1c.7: Responsive Layout (Mobile)

- Board takes full width on mobile
- Side panel collapses below the board
- Touch-friendly move input (tap-tap instead of drag)
- Controls and history in collapsible accordion
- Eval bar moves to horizontal bar above/below board on mobile

### Task 1c.8: Polish & Edge Cases

- Animated piece movement (react-chessboard built-in)
- Premove support (optional — queue a move while engine thinks)
- Draw detection: 50-move rule, threefold repetition, insufficient material
- Stalemate detection
- Game over modal with result and option to start new game
- Error boundaries for WASM crashes

### Phase 1c Success Criteria

- [ ] Analysis mode works (free play, continuous eval)
- [ ] FEN import/export works
- [ ] PGN export downloads valid PGN file
- [ ] Keyboard navigation works
- [ ] Loading state shown during engine init
- [ ] Responsive on mobile (board + controls usable)
- [ ] Draw/stalemate correctly detected and displayed

### Phase 1c Estimated Time: 2-3 days

---

## Phase 1d: Integration & Deploy (1 day)

### Goal
Connect the chess frontend to chess.vaishakmenon.com and deploy.

### Task 1d.1: Subdomain Configuration

- Configure `chess.vaishakmenon.com` DNS (CNAME or A record to your VPS)
- Set up reverse proxy (nginx/Caddy) to route to the Next.js frontend
- If deploying on the same VPS as the portfolio: add a new service for the chess frontend

### Task 1d.2: Portfolio Site Integration

- Add "Chess" link to the site navigation on vaishakmenon.com
- Add a project card for Vantage on the homepage/projects page
- Include a screenshot or embedded preview

### Task 1d.3: SEO & Metadata

```typescript
// frontend/src/app/chess/layout.tsx
export const metadata = {
  title: 'Play Chess Against Vantage | Vaishak Menon',
  description: 'Play chess against Vantage, a Rust chess engine running in your browser via WebAssembly.',
  openGraph: {
    title: 'Vantage Chess',
    description: 'A Rust chess engine in your browser',
    url: 'https://chess.vaishakmenon.com',
  },
};
```

### Task 1d.4: Production Build & Optimization

```bash
cd frontend
npm run build
```

- Verify WASM loads correctly in production build
- Check bundle size (WASM should be loaded async, not in main bundle)
- Test service worker caching for WASM binary (1.6MB — worth caching)
- Add `Cache-Control` headers for WASM file (immutable, long TTL)

### Task 1d.5: Cross-Browser Testing

Test on:
- Chrome (primary target)
- Firefox
- Safari (WebAssembly + Web Workers support)
- Mobile Chrome / Safari

Known risks:
- Safari has stricter WASM memory limits — test 64MB TT
- Some browsers may block Web Workers from `file://` (only matters for local dev)

### Task 1d.6: Deploy

Deployment strategy depends on your VPS setup:
- **Option A:** Build and serve via nginx (static export or `next start`)
- **Option B:** Docker container on your VPS
- **Option C (future):** k3s pod after migration

Minimum viable deploy:
```bash
cd frontend
npm run build
# Copy .next/ and public/ to VPS
# Run with: npx next start -p 3001
# Reverse proxy chess.vaishakmenon.com → localhost:3001
```

### Phase 1d Success Criteria

- [ ] chess.vaishakmenon.com loads the chess UI
- [ ] WASM engine initializes and plays correctly
- [ ] Navigation from portfolio site works
- [ ] Metadata/OG tags render correctly
- [ ] Works on Chrome, Firefox, Safari
- [ ] Works on mobile browsers
- [ ] WASM binary is cached after first load

### Phase 1d Estimated Time: 1 day

---

## Task Summary

| Phase | Task | Description | Est. Time |
|-------|------|-------------|-----------|
| **1a** | 1a.1 | Next.js scaffolding + deps | 1 hr |
| | 1a.2 | TypeScript type definitions | 30 min |
| | 1a.3 | Engine adapter + Worker + WasmEngine | 2 hr |
| | 1a.4 | useGameState hook | 1.5 hr |
| | 1a.5 | useEngine hook | 1 hr |
| | 1a.6 | Board component | 1.5 hr |
| | 1a.7 | ChessGame orchestrator + page | 1.5 hr |
| | 1a.8 | Theme & styling | 30 min |
| **1b** | 1b.1 | Eval bar | 1.5 hr |
| | 1b.2 | Move history panel | 1.5 hr |
| | 1b.3 | Engine info display | 45 min |
| | 1b.4 | Game controls | 1 hr |
| | 1b.5 | Difficulty selector | 1 hr |
| | 1b.6 | Clock display + useClock | 1.5 hr |
| | 1b.7 | Desktop layout assembly | 1 hr |
| **1c** | 1c.1 | Analysis mode toggle | 1.5 hr |
| | 1c.2 | FEN import/export | 45 min |
| | 1c.3 | PGN export | 1 hr |
| | 1c.4 | PV line display | 1 hr |
| | 1c.5 | Keyboard navigation | 30 min |
| | 1c.6 | Loading state | 45 min |
| | 1c.7 | Responsive layout | 2 hr |
| | 1c.8 | Polish & edge cases | 2 hr |
| **1d** | 1d.1 | Subdomain config | 30 min |
| | 1d.2 | Portfolio integration | 30 min |
| | 1d.3 | SEO metadata | 15 min |
| | 1d.4 | Production build | 30 min |
| | 1d.5 | Cross-browser testing | 1 hr |
| | 1d.6 | Deploy | 1 hr |
| | **Total** | | **~28-30 hrs** |

---

## Component Dependency Graph

```
page.tsx
  └── ChessGame.tsx
        ├── useEngine (→ WasmEngine → Worker → WASM)
        ├── useGameState (→ chess.js)
        ├── useClock
        ├── Board.tsx (react-chessboard)
        ├── EvalBar.tsx
        ├── MoveHistory.tsx
        ├── EngineInfo.tsx
        ├── GameControls.tsx
        ├── ClockDisplay.tsx
        ├── DifficultySelector.tsx
        ├── ModeToggle.tsx
        ├── FenInput.tsx
        ├── PgnExport.tsx
        └── PVLines.tsx
```

---

## Feeding to Claude Code

For each phase, you can give Claude Code a single prompt:

**Phase 1a:**
```
Read @phase-1-frontend.md and implement Phase 1a tasks 1a.1 through 1a.8 sequentially.
Use the frontend-patterns skill for component conventions.
After creating the project scaffold, verify npm run dev works.
After wiring up the engine, verify the play loop works end-to-end.
```

**Phase 1b:**
```
Read @phase-1-frontend.md and implement Phase 1b tasks 1b.1 through 1b.7.
All components should match the dark theme in lib/theme.ts.
Verify the desktop layout matches the wireframe.
```

**Phase 1c:**
```
Read @phase-1-frontend.md and implement Phase 1c tasks 1c.1 through 1c.8.
Test analysis mode, FEN import, PGN export, and responsive layout.
```

**Phase 1d:**
```
Read @phase-1-frontend.md and implement Phase 1d tasks 1d.1 through 1d.6.
Configure chess.vaishakmenon.com deployment.
```

---

## Known Limitations for v1

- **No search info streaming**: The WASM API returns only the final bestmove + score. Eval bar updates after search completes, not during. Full streaming requires the `on_info` callback refactor (planned for Phase 2).
- **Single PV line only**: Multi-PV requires engine changes.
- **No saved games**: Stateless — no user accounts or game persistence.
- **No sound effects**: Silent for v1.
- **No board editor**: FEN import only (no drag-and-drop piece placement).
- **~7s engine init**: Users see a loading screen on first visit. Subsequent visits benefit from browser WASM cache.
