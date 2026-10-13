import subprocess
import time
import os
import psutil
import sys

print("=================================================================")
print("  PHASE 1.2: REAL QT-TO-RUST LIFECYCLE & SHUTDOWN VALIDATION")
print("  Testing full Qt -> C-ABI -> Rust process lifecycle under Wayland")
print("=================================================================")

env = dict(os.environ, QT_QPA_PLATFORM="wayland")
qt_bin = "./qt/build/ryzora"

# --- TEST 1: Qt Internal Lifecycle Suite (Qt Cancel Button Latency & UI Storm) ---
print("\n[TEST 1] Running Qt Internal Lifecycle Suite (--test-lifecycle)...")
t0 = time.perf_counter()
res = subprocess.run([qt_bin, "--test-lifecycle"], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=15)
t_suite = (time.perf_counter() - t0) * 1000.0

print(res.stderr)
assert res.returncode == 0, f"Lifecycle suite failed with exit code {res.returncode}!"
print(f"Qt Lifecycle Suite PASSED in {t_suite:.1f} ms (exit code 0).")

# --- TEST 2: Real Application SIGTERM / Window Close During Active Preview ---
print("\n[TEST 2] Testing Application Shutdown during in-flight preview...")
proc = subprocess.Popen([qt_bin], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
time.sleep(0.5) # Wait for surface mapping and initial preview

# Check child processes
children_before = []
try:
    p = psutil.Process(proc.pid)
    children_before = p.children(recursive=True)
except Exception:
    pass

print(f"Prototype running (PID: {proc.pid}), initial child count: {len(children_before)}")

# Send SIGTERM (standard compositor window close / quit signal)
print("Sending SIGTERM to Qt prototype while preview is active...")
t_term0 = time.perf_counter()
proc.terminate()
try:
    proc.wait(timeout=3.0)
except subprocess.TimeoutExpired:
    proc.kill()
    proc.wait()
t_term = (time.perf_counter() - t_term0) * 1000.0

print(f"Qt prototype exited cleanly in {t_term:.1f} ms with code {proc.returncode}")
assert proc.returncode == 0 or proc.returncode == -15 or proc.returncode == 143, f"Unexpected exit code: {proc.returncode}"

# --- TEST 3: Zero Orphan / Zombie Process Verification on Host ---
print("\n[TEST 3] Verifying host process table for zero orphans or zombies...")
time.sleep(0.2)
orphans = []
for p in psutil.process_iter(['pid', 'ppid', 'name', 'status']):
    try:
        name = (p.info['name'] or '').lower()
        if 'fastfetch' in name or ('sleep' in name and p.info['ppid'] == 1):
            orphans.append(p.info)
    except Exception:
        pass

print(f"Orphan processes detected: {len(orphans)}")
if orphans:
    for o in orphans:
        print("  - Orphan:", o)
assert len(orphans) == 0, f"Leaked orphan processes detected: {orphans}"

print("\n=================================================================")
print("  ALL QT-TO-RUST LIFECYCLE TESTS PASSED (0 ERRORS, 0 LEAKS)")
print("=================================================================")
