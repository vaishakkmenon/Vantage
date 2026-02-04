#!/usr/bin/env python3
import sys
import subprocess
import shutil
import datetime
import re
from pathlib import Path

# =============================================================================
# CONFIGURATION
# =============================================================================
HERO_ENGINE = "/workspace/backend/target/release/vantage"
BASE_RESULTS_DIR = "/results"
OPENING_BOOK = "/openings/UHO_Lichess_4852_v1.epd"
TIME_CONTROL = "20+0.2" 
GAMES_PER_ROUND = 20       

# The Gauntlet Roster
OPPONENTS = [
    # --- Level 1: Sanity Check ---
    # Now built-in at /engines/tscp
    {"name": "TSCP",      "cmd": "/engines/tscp",      "elo": "1700", "proto": "xboard", "use_book": False},
    
    # --- Level 2: Club Player ---
    {"name": "Fairymax",  "cmd": "/engines/fairymax",  "elo": "2000", "proto": "xboard", "use_book": True},
    
    # --- Level 3: Master Strength ---
    {"name": "Crafty",    "cmd": "/engines/crafty",    "elo": "2300", "proto": "xboard", "use_book": True},
    {"name": "Phalanx",   "cmd": "/engines/phalanx",   "elo": "2400", "proto": "xboard", "use_book": True},
    
    # --- Level 4: Grandmaster Strength ---
    # Now built-in at /engines/fruit
    {"name": "Fruit",     "cmd": "/engines/fruit",     "elo": "2800", "proto": "uci",    "use_book": True},
    
    # --- Level 5: Superhuman ---
    {"name": "Ethereal",  "cmd": "/engines/ethereal",  "elo": "3050", "proto": "uci",    "use_book": True},
    
    # --- Level 6: God Mode ---
    {"name": "Stockfish", "cmd": "/engines/stockfish", "elo": "3500", "proto": "uci",    "use_book": True}
]

# =============================================================================
# LOGIC
# =============================================================================
def run_match(opponent, run_dir):
    print(f"\n>>> MATCH STARTING: Vantage vs {opponent['name']} ({opponent['elo']} ELO)")
    
    pgn_file = f"{run_dir}/Vantage_vs_{opponent['name']}.pgn"
    
    cmd = [
        "cutechess-cli",
        "-engine", f"name=Vantage", f"cmd={HERO_ENGINE}", "proto=uci",
        "-engine", f"name={opponent['name']}", f"cmd={opponent['cmd']}", f"proto={opponent['proto']}",
        "-each", f"tc={TIME_CONTROL}",
        "-pgnout", pgn_file,
        "-rounds", str(GAMES_PER_ROUND),
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

def save_summary(results, run_dir):
    summary_path = f"{run_dir}/summary.txt"
    timestamp = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    
    output = []
    output.append(f"GAUNTLET SUMMARY REPORT")
    output.append(f"Date: {timestamp}")
    output.append(f"Engine: Vantage")
    output.append(f"-" * 65)
    output.append(f"{'OPPONENT':<15} {'ELO':<10} {'SCORE':<10} {'RESULT':<15} {'VERDICT'}")
    output.append(f"-" * 65)
    
    total_games = 0
    total_points = 0.0

    for r in results:
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

def main():
    print("--- BUILDING VANTAGE (RELEASE MODE) ---")
    try:
        subprocess.run(["cargo", "build", "--release"], cwd="/workspace/backend", check=True)
    except subprocess.CalledProcessError:
        print("Build failed!")
        sys.exit(1)

    timestamp = datetime.datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
    run_dir = f"{BASE_RESULTS_DIR}/{timestamp}"
    Path(run_dir).mkdir(parents=True, exist_ok=True)
    print(f"--- CREATED RUN DIRECTORY: {run_dir} ---")
    
    results = []
    for opp in OPPONENTS:
        res = run_match(opp, run_dir)
        results.append(res)
        
    save_summary(results, run_dir)

if __name__ == "__main__":
    main()