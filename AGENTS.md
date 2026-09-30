# Ryzora — Token Efficiency / Compact Agent Policy

You are working on the Ryzora project.

IMPORTANT:
Be concise by default. Optimize for useful engineering work, not narration.

==================================================
1. NO STEP-BY-STEP NARRATION
==================================================

Do NOT narrate every command.

Bad:
"Now I will inspect the file..."
"Next I will run grep..."
"Now I am going to check..."
"I have launched..."
"I will inspect the result..."

Just execute the command and inspect the result.

Only explain when:
- a decision is important
- a safety issue is discovered
- an implementation choice needs approval
- a test fails
- a blocker exists

==================================================
2. MINIMIZE COMMAND OUTPUT
==================================================

Never dump large files unless necessary.

Prefer:
sed -n 'START,ENDp' file
grep -n 'pattern' file
rg 'pattern' file
git diff -- file
git diff --stat

Avoid:
cat huge_file
full file dumps
repeatedly printing the same sections
printing entire test logs

When a command produces large output, filter it.

==================================================
3. DO NOT RE-READ WHAT YOU ALREADY KNOW
==================================================

Keep a compact internal working summary.

Once a file/section has been inspected, do not repeatedly
read the same section unless it changed.

Do not repeatedly inspect:
- the same struct
- the same function
- the same config
- the same test output

Only re-read after modifying it or when verification requires it.

==================================================
4. BATCH INSPECTION
==================================================

When several related facts are needed, inspect them in ONE command.

Prefer:
python3 - <<'PY'
...
PY
or one shell command containing several focused checks.

Do not make 10 sequential commands when 1-2 commands can answer
the same question.

==================================================
5. NO REDUNDANT TEST RUNS
==================================================

Do not repeatedly run the same test suite after every tiny edit.

Preferred:
1. edit
2. focused test
3. fix failures
4. focused test again
5. final verification:
   - frontend tests
   - TypeScript
   - build
   - targeted Rust
   - full Rust

If a test is already known to pass and nothing affecting it changed,
do not rerun it.

==================================================
6. TEST OUTPUT SHOULD BE SUMMARIZED
==================================================

Do not paste hundreds of passing test lines.

Extract only:

PASS:
- frontend: 225/225
- Rust: 599/599
- SDDM: 78/78
- TypeScript: 0 errors
- build: success

For failures, show the relevant failure and enough context to diagnose it.

==================================================
7. COMPACT FINAL REPORT
==================================================

Final reports should normally be <= 20 lines.

Use:

Status: PASS / BLOCKED

Changed:
- file1
- file2

Implemented:
- short bullet
- short bullet

Verification:
- tests: X/Y
- TypeScript: pass
- build: pass
- Rust: X/Y

Safety:
- relevant invariant verified

Blockers:
- None
or
- exact blocker

Commit:
- Not committed

Do NOT produce a long essay unless explicitly requested.

==================================================
8. NEVER REPEAT THE USER'S REQUIREMENTS
==================================================

Do not restate the entire task before implementing it.

Do not repeat the architecture specification in every response.

Refer to existing project invariants briefly.

==================================================
9. DO NOT CREATE LARGE SCRATCH SCRIPTS UNNECESSARILY
==================================================

Prefer direct edits when small.

For larger transformations, use a short script.

Delete temporary scripts after use.

==================================================
10. NO FAKE PROGRESS
==================================================

Never say:
"I have launched X and will inspect it later."

Actually wait for the result when possible.

Do not generate conversational filler around long-running commands.

==================================================
11. STOP WHEN THE TASK IS COMPLETE
==================================================

Once:
- implementation is correct
- required tests pass
- requested verification is complete

STOP.

Do not perform unrelated exploratory checks.
Do not "improve" unrelated code.
Do not refactor working architecture without a concrete reason.

==================================================
12. SAFETY OVERRIDES TOKEN OPTIMIZATION
==================================================

Token efficiency must NEVER remove required safety verification.

For system integration:
- ownership checks remain
- rollback verification remains
- real-host validation remains when required
- hashes remain
- fail-closed behavior remains

Use concise output, not reduced safety.

==================================================
13. RYZORA-SPECIFIC RULE
==================================================

Before editing, identify:
A. exact files involved
B. exact invariant being changed
C. minimum implementation required

Then implement only that scope.

==================================================
14. FINAL RESPONSE FORMAT
==================================================

Always finish with:

Status: <PASS/BLOCKED>

Changed: <short list>

Tests: <compact results>

Blockers: <None/exact blocker>

Commit: <not committed / hash>
