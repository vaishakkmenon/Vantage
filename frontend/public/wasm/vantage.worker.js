// vantage.worker.js
// Runs in a Web Worker — all WASM calls happen here, never on the main thread.
// Communication with the React hook is via postMessage.

let engine = null;

self.onmessage = async (e) => {
  const { id, type, payload } = e.data;

  try {
    switch (type) {
      case 'init': {
        const module = await import('/wasm/vantage.js');
        await module.default();
        engine = new module.VantageEngine();
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
        const result = JSON.parse(engine.make_move(payload.move));
        respond(id, 'move_result', result);
        break;
      }

      case 'is_move_legal': {
        const legal = engine.is_move_legal(payload.move);
        respond(id, 'legal', { legal });
        break;
      }

      case 'search': {
        let result;
        if (payload.depth) {
          result = JSON.parse(engine.go_depth(payload.depth));
        } else if (payload.movetime) {
          result = JSON.parse(engine.go_movetime(payload.movetime));
        } else {
          result = JSON.parse(engine.go_depth(10));
        }
        respond(id, 'search_result', result);
        break;
      }

      case 'get_legal_moves': {
        const moves = JSON.parse(engine.get_legal_moves());
        respond(id, 'legal_moves', { moves });
        break;
      }

      case 'get_legal_moves_for_square': {
        const moves = JSON.parse(engine.get_legal_moves_for_square(payload.square));
        respond(id, 'legal_moves_for_square', { moves });
        break;
      }

      case 'get_fen': {
        respond(id, 'fen', { fen: engine.get_fen() });
        break;
      }

      case 'get_game_status': {
        const status = engine.get_game_status();
        respond(id, 'game_status', { status });
        break;
      }

      case 'side_to_move': {
        respond(id, 'side', { side: engine.side_to_move() });
        break;
      }

      default:
        respond(id, 'error', {}, `Unknown message type: ${type}`);
    }
  } catch (err) {
    respond(id, 'error', {}, err.message || 'Unknown worker error');
  }
};

function respond(id, type, payload, error) {
  self.postMessage({ id, type, payload, error });
}