import shutil
import subprocess
import os

# The roster of engines we expect to find
engines = [
    {"name": "Stockfish", "cmd": "stockfish"},      # System (Manual install)
    {"name": "Ethereal",  "cmd": "ethereal-chess"}, # System (apt)
    {"name": "Phalanx",   "cmd": "phalanx"},        # System (apt)
    {"name": "Fairymax",  "cmd": "fairymax"},       # System (apt)
    {"name": "Crafty",    "cmd": "crafty"},         # System (apt)
    {"name": "Fruit",     "cmd": "/engines/fruit"}, # Local Bind Mount
    {"name": "TSCP",      "cmd": "/engines/tscp"}   # Local Bind Mount
]

print(f"{'ENGINE':<12} {'STATUS':<10} {'DETAILS'}")
print("-" * 60)

for eng in engines:
    cmd = eng["cmd"]
    
    # 1. Check if the command/file exists
    path = shutil.which(cmd)
    
    # If shutil.which() failed, it might be a direct file path that isn't in PATH
    if not path and os.path.exists(cmd):
        path = cmd

    if not path:
        print(f"{eng['name']:<12} \033[91mMISSING\033[0m    (Command '{cmd}' not found)")
        continue

    # 2. Check Permissions (Is it executable?)
    if not os.access(path, os.X_OK):
        print(f"{eng['name']:<12} \033[91mPERM_ERR\033[0m   (Found at {path} but not executable)")
        print(f"             -> Fix: chmod +x {path}")
        continue

    # 3. Liveliness Check (Can we actually run it?)
    try:
        # We send 'quit' so it exits immediately after starting
        subprocess.run(
            [path], 
            input=b"quit\n", 
            stdout=subprocess.DEVNULL, 
            stderr=subprocess.DEVNULL, 
            timeout=1
        )
        print(f"{eng['name']:<12} \033[92mOK\033[0m         {path}")
    except subprocess.TimeoutExpired:
        # If it hangs but launched, it's probably fine (just ignored 'quit')
        print(f"{eng['name']:<12} \033[92mOK (Slow)\033[0m  {path}")
    except Exception as e:
        print(f"{eng['name']:<12} \033[91mCRASHED\033[0m    {e}")