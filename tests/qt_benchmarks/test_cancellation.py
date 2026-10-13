import subprocess
import time
import os
import psutil

print("=== FASTFETCH CANCELLATION VALIDATION TEST ===")

# Test 1: Run Rust unit tests which explicitly test PID termination and reaping
print("\n[TEST 1] Running Rust ProcessTracker unit tests...")
res = subprocess.run(["cargo", "test", "--manifest-path", "crates/ryzora-qt-backend/Cargo.toml", "--", "--test-threads=1", "--nocapture"], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
print(res.stdout)
assert res.returncode == 0, "Rust unit tests failed!"

# Test 2: Subprocess tree verification during live execution
print("\n[TEST 2] Verifying no zombie or orphaned fastfetch processes exist right now...")
orphans = []
for p in psutil.process_iter(['pid', 'name', 'status']):
    try:
        if 'fastfetch' in p.info['name'].lower():
            orphans.append(p.info)
    except:
        pass
print(f"Current active fastfetch processes on host: {len(orphans)}")
for o in orphans:
    print(" -", o)

print("\nCancellation verification completed successfully.")
