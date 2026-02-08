// test-worker.js
import init, { VantageEngine } from './pkg/vantage.js';

let engine = null;

self.onmessage = async function(e) {
    const { type, payload } = e.data;

    switch (type) {
        case 'init':
            await init();
            engine = new VantageEngine();
            self.postMessage({ type: 'ready' });
            break;

        case 'new_game':
            engine.new_game();
            self.postMessage({ type: 'new_game_ok' });
            break;

        case 'set_position':
            if (payload.fen) {
                engine.set_position_fen(payload.fen);
            } else {
                engine.set_position_startpos(payload.moves || '');
            }
            self.postMessage({
                type: 'position_set',
                fen: engine.get_fen(),
                side: engine.side_to_move(),
                legal_moves: JSON.parse(engine.get_legal_moves()),
            });
            break;

        case 'apply_move': {
            const ok = engine.apply_move(payload.move);
            self.postMessage({
                type: 'move_applied',
                success: ok,
                fen: engine.get_fen(),
                side: engine.side_to_move(),
                legal_moves: JSON.parse(engine.get_legal_moves()),
            });
            break;
        }

        case 'go': {
            let result;
            if (payload.depth) {
                result = engine.go_depth(payload.depth);
            } else if (payload.movetime) {
                result = engine.go_movetime(payload.movetime);
            }
            self.postMessage({
                type: 'search_result',
                result: JSON.parse(result),
            });
            break;
        }
    }
};
