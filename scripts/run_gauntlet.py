#!/usr/bin/env python3
import sys
import subprocess
import shutil
from pathlib import Path

# Configuration
HERO_ENGINE = "/workspace/target/release/vantage"  # Path to your compiled engine
RESULTS_DIR = "/results/pgn"
OPENING_BOOK = "/openings/UHO_Lichess_4852_v1.epd"
TIME_CONTROL = "20+0.2"  # 20 seconds + 0.2s increment
GAMES_PER_ROUND = 20     # Games per opponent

# The Gauntlet Roster
OPPONENTS = [
    # --- Level 1: The Sanity Checks ---
    # TSCP is the standard "bug check" engine. 
    {"name": "TSCP",      "cmd": "/engines/tscp",      "elo": "1700"},

    # --- Level 2: Club Player ---
    # Fairymax is a solid tactical check.
    {"name": "Fairymax",  "cmd": "fairymax",           "elo": "2000"},

    # --- Level 3: Master Strength (The Mid-Field) ---
    # Crafty and Phalanx are legendary master-level engines.
    {"name": "Crafty",    "cmd": "crafty",             "elo": "2300"},
    {"name": "Phalanx",   "cmd": "phalanx",            "elo": "2400"},

    # --- Level 4: Grandmaster Strength ---
    # Fruit is the gatekeeper to "Super Engine" status.
    {"name": "Fruit",     "cmd": "/engines/fruit",     "elo": "2800"},

    # --- Level 5: Superhuman ---
    # Ethereal is a modern powerhouse.
    # {"name": "Ethereal",  "cmd": "ethereal-chess",     "elo": "3000"},

    # --- Level 6: God Mode ---
    # The ceiling. You will likely score 0% here for a long time.
    {"name": "Stockfish", "cmd": "stockfish",          "elo": "3500"}
]

def run_match(opponent):
    print(f"\n=== MATCH: Vantage vs {opponent['name']} ({opponent['elo']} ELO) ===")
    
    pgn_file = f"{RESULTS_DIR}/vantage_vs_{opponent['name']}.pgn"
    
    cmd = [
        "cutechess-cli",
        "-engine", f"name=Vantage", f"cmd={HERO_ENGINE}", "proto=uci",
        "-engine", f"name={opponent['name']}", f"cmd={opponent['cmd']}",
        "-each", f"tc={TIME_CONTROL}", "proto=uci",
        "-pgnout", pgn_file,
        "-rounds", str(GAMES_PER_ROUND),
        "-concurrency", "2",
        "-openings", f"file={OPENING_BOOK}", "format=epd", "order=random",
    ]

    try:
        subprocess.run(cmd, check=True)
        print(f"Match complete. Results saved to {pgn_file}")
    except subprocess.CalledProcessError:
        print(f"ERROR: Match against {opponent['name']} failed.")

def main():
    # 1. Compile latest version
    print("Building Vantage...")
    subprocess.run(["cargo", "build", "--release"], cwd="/workspace", check=True)

    # 2. Run the Gauntlet
    Path(RESULTS_DIR).mkdir(parents=True, exist_ok=True)
    
    for opp in OPPONENTS:
        # Check if opponent exists
        if not shutil.which(opp["cmd"]):
            print(f"Skipping {opp['name']} (Command not found)")
            continue
            
        run_match(opp)

if __name__ == "__main__":
    main()