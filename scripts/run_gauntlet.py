#!/usr/bin/env python3
import sys
import subprocess
import shutil
import datetime
import re
import argparse
from pathlib import Path

# =============================================================================
# CONFIGURATION
# =============================================================================
HERO_ENGINE = "/workspace/backend/target/release/vantage"
BASE_RESULTS_DIR = "/results"
OPENING_BOOK = "/openings/UHO_Lichess_4852_v1.epd"
TIME_CONTROL = "20+0.2"
DEFAULT_GAMES_PER_ROUND = 20

# The Gauntlet Roster
ALL_OPPONENTS = {
    # --- Level 1: Sanity Check (~1700) ---
    "tscp": {"name": "TSCP", "cmd": "/engines/tscp", "elo": "1724", "proto": "xboard", "use_book": False},

    # --- Level 2: Club Player (~1900) ---
    "fairymax": {"name": "Fairymax", "cmd": "/engines/fairymax", "elo": "1890", "proto": "xboard", "use_book": True},

    # --- Level 3: Expert (~2600) ---
    "phalanx": {"name": "Phalanx", "cmd": "/engines/phalanx", "elo": "2600", "proto": "xboard", "use_book": True},

    # --- Level 4: Strong Expert (~2700) ---
    "fruit": {"name": "Fruit", "cmd": "/engines/fruit", "elo": "2700", "proto": "uci", "use_book": True},

    # --- Level 5: Near-Master Engine (~2950) ---
    "crafty": {"name": "Crafty", "cmd": "/engines/crafty", "elo": "2950", "proto": "xboard", "use_book": True},

    # --- Level 6: Superhuman (~3450) ---
    "ethereal": {"name": "Ethereal", "cmd": "/engines/ethereal", "elo": "3450", "proto": "uci", "use_book": True},

    # --- Level 7: God Mode (~3620) ---
    "stockfish": {"name": "Stockfish", "cmd": "/engines/stockfish", "elo": "3620", "proto": "uci", "use_book": True},
}

# Default opponents when running full gauntlet (no --opponent specified)
DEFAULT_OPPONENTS = ["tscp", "fairymax", "phalanx", "fruit", "crafty", "ethereal", "stockfish"]

# =============================================================================
# LOGIC
# =============================================================================
def run_match(opponent, run_dir, games_per_round):
    print(f"\n>>> MATCH STARTING: Vantage vs {opponent['name']} ({opponent['elo']} ELO)")

    pgn_file = f"{run_dir}/Vantage_vs_{opponent['name']}.pgn"

    cmd = [
        "cutechess-cli",
        "-engine", f"name=Vantage", f"cmd={HERO_ENGINE}", "proto=uci",
        "-engine", f"name={opponent['name']}", f"cmd={opponent['cmd']}", f"proto={opponent['proto']}",
        "-each", f"tc={TIME_CONTROL}",
        "-pgnout", pgn_file,
        "-rounds", str(games_per_round),
        "-concurrency", "2",
        "-repeat"
    ]

    if opponent['use_book']:
        cmd.extend(["-openings", f"file={OPENING_BOOK}", "format=epd", "order=random"])

    # Variables to track score
    wins, losses, draws = 0, 0, 0

    try:
        # Run process and stream output line by line so user sees progress
        with subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, cwd="/tmp") as proc:
            for line in proc.stdout:
                print(line, end='') # Echo to console

                # Parse Score Line: "Score of Vantage vs TSCP: 18 - 2 - 0  [0.900] 20"
                if "Score of Vantage vs" in line:
                    match = re.search(r"(\d+) - (\d+) - (\d+)", line)
                    if match:
                        wins = int(match.group(1))
                        losses = int(match.group(2))
                        draws = int(match.group(3))

        if proc.returncode != 0:
            print(f"!!! ERROR: Match against {opponent['name']} failed.")
            return None

        # Clean up
        if opponent['proto'] == "xboard":
            subprocess.run("rm -f /tmp/*.001 /tmp/*.log", shell=True)

        return {"name": opponent['name'], "elo": opponent['elo'], "w": wins, "l": losses, "d": draws}

    except Exception as e:
        print(f"!!! CRITICAL ERROR: {e}")
        return None

def load_existing_results(run_dir):
    """Load existing results from summary.txt if it exists."""
    summary_path = Path(run_dir) / "summary.txt"
    results = {}

    if not summary_path.exists():
        return results

    with open(summary_path) as f:
        for line in f:
            # Parse lines like: "Crafty          2300       +5 -10 =5    50.0% (10/20)   PASS"
            match = re.match(r'^(\w+)\s+(\d+)\s+\+(\d+)\s+-(\d+)\s+=(\d+)', line)
            if match:
                name = match.group(1)
                results[name] = {
                    "name": name,
                    "elo": match.group(2),
                    "w": int(match.group(3)),
                    "l": int(match.group(4)),
                    "d": int(match.group(5)),
                }

    return results

def merge_results(existing, new_results):
    """Merge new results into existing, accumulating scores."""
    merged = existing.copy()

    for r in new_results:
        if r is None:
            continue
        name = r['name']
        if name in merged:
            # Accumulate scores
            merged[name]['w'] += r['w']
            merged[name]['l'] += r['l']
            merged[name]['d'] += r['d']
        else:
            merged[name] = r.copy()

    return merged

def save_summary(results, run_dir):
    """Save summary to file. results can be a dict or list."""
    summary_path = f"{run_dir}/summary.txt"
    timestamp = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")

    # Convert dict to list if needed
    if isinstance(results, dict):
        results_list = list(results.values())
    else:
        results_list = results

    # Sort by ELO for consistent ordering
    results_list = sorted([r for r in results_list if r], key=lambda x: int(x['elo']))

    output = []
    output.append(f"GAUNTLET SUMMARY REPORT")
    output.append(f"Date: {timestamp}")
    output.append(f"Engine: Vantage")
    output.append(f"-" * 65)
    output.append(f"{'OPPONENT':<15} {'ELO':<10} {'SCORE':<10} {'RESULT':<15} {'VERDICT'}")
    output.append(f"-" * 65)

    total_games = 0
    total_points = 0.0

    for r in results_list:
        if r is None: continue

        # Calculate points (Win = 1, Draw = 0.5)
        points = r['w'] + (r['d'] * 0.5)
        games = r['w'] + r['l'] + r['d']
        percentage = (points / games) * 100 if games > 0 else 0

        score_str = f"+{r['w']} -{r['l']} ={r['d']}"
        result_str = f"{percentage:.1f}% ({points}/{games})"

        # Verdict logic
        if percentage >= 50: verdict = "PASS"
        else: verdict = "FAIL"

        output.append(f"{r['name']:<15} {r['elo']:<10} {score_str:<10} {result_str:<15} {verdict}")

        total_games += games
        total_points += points

    output.append(f"-" * 65)
    if total_games > 0:
        total_perf = (total_points / total_games) * 100
        output.append(f"TOTAL PERFORMANCE: {total_perf:.1f}% ({total_points}/{total_games})")

    # Print to console
    print("\n" + "\n".join(output))

    # Save to file
    with open(summary_path, "w") as f:
        f.write("\n".join(output))

    print(f"\n>>> SUMMARY SAVED TO: {summary_path}")

def parse_args():
    parser = argparse.ArgumentParser(
        description="Run Vantage chess engine gauntlet against various opponents.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s                           # Run against default opponents (crafty)
  %(prog)s -o crafty                 # Run against Crafty only
  %(prog)s -o crafty -o tscp         # Run against Crafty and TSCP
  %(prog)s -o crafty -r 50           # Run 50 games against Crafty
  %(prog)s -o crafty -d /results/existing  # Append to existing results
  %(prog)s --list                    # List available opponents

Available opponents: """ + ", ".join(ALL_OPPONENTS.keys())
    )

    parser.add_argument(
        "-o", "--opponent",
        action="append",
        dest="opponents",
        metavar="NAME",
        help="Opponent engine to play against (can be specified multiple times)"
    )

    parser.add_argument(
        "-d", "--output-dir",
        metavar="DIR",
        help="Use existing results directory (results will be merged)"
    )

    parser.add_argument(
        "-r", "--rounds",
        type=int,
        default=DEFAULT_GAMES_PER_ROUND,
        metavar="N",
        help=f"Number of games per opponent (default: {DEFAULT_GAMES_PER_ROUND})"
    )

    parser.add_argument(
        "--no-build",
        action="store_true",
        help="Skip cargo build step"
    )

    parser.add_argument(
        "--list",
        action="store_true",
        help="List available opponents and exit"
    )

    return parser.parse_args()

def main():
    args = parse_args()

    # List opponents and exit
    if args.list:
        print("Available opponents:")
        for key, opp in ALL_OPPONENTS.items():
            print(f"  {key:<12} - {opp['name']:<12} (ELO: {opp['elo']}, {opp['proto']})")
        sys.exit(0)

    # Build engine
    if not args.no_build:
        print("--- BUILDING VANTAGE (RELEASE MODE) ---")
        try:
            subprocess.run(["cargo", "build", "--release"], cwd="/workspace/backend", check=True)
        except subprocess.CalledProcessError:
            print("Build failed!")
            sys.exit(1)
    else:
        print("--- SKIPPING BUILD (--no-build) ---")

    # Determine output directory
    if args.output_dir:
        run_dir = args.output_dir
        Path(run_dir).mkdir(parents=True, exist_ok=True)
        print(f"--- USING EXISTING DIRECTORY: {run_dir} ---")
    else:
        timestamp = datetime.datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
        run_dir = f"{BASE_RESULTS_DIR}/{timestamp}"
        Path(run_dir).mkdir(parents=True, exist_ok=True)
        print(f"--- CREATED RUN DIRECTORY: {run_dir} ---")

    # Load existing results if any
    existing_results = load_existing_results(run_dir)
    if existing_results:
        print(f"--- LOADED {len(existing_results)} EXISTING RESULT(S) ---")

    # Determine which opponents to run against
    if args.opponents:
        opponent_keys = []
        for opp_name in args.opponents:
            opp_lower = opp_name.lower()
            if opp_lower not in ALL_OPPONENTS:
                print(f"!!! ERROR: Unknown opponent '{opp_name}'")
                print(f"    Available: {', '.join(ALL_OPPONENTS.keys())}")
                sys.exit(1)
            opponent_keys.append(opp_lower)
    else:
        opponent_keys = DEFAULT_OPPONENTS

    opponents = [ALL_OPPONENTS[k] for k in opponent_keys]

    # Run matches
    new_results = []
    for opp in opponents:
        res = run_match(opp, run_dir, args.rounds)
        new_results.append(res)

    # Merge and save
    merged_results = merge_results(existing_results, new_results)
    save_summary(merged_results, run_dir)

if __name__ == "__main__":
    main()
