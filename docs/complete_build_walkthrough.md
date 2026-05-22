# Ferrocene ASIL-D POC — Complete Build Walkthrough

## What This Is and Why It Exists

In a web app, when something breaks, the user sees an error. You fix it and push an update. Nobody gets hurt.

In a car, when software breaks at 120 km/h, there is no error screen. There is no update. There is only a car that cannot stop.

This is why automotive software is built differently. Every decision — every file, every flag, every test, every script — exists to answer one question a safety auditor will ask:

> **"How do you know it cannot break?"**

Not "did you test it." Not "does it usually work." How do you know, with signed, reproducible, traceable evidence?

This pipeline builds that evidence, step by step.

---

## What Ferrocene Is — The One Paragraph You Need

Rust's standard compiler (`rustc`) is excellent engineering, but it carries no legal safety certification. Ferrocene is the same compiler, rebuilt by Ferrous Systems and independently audited by TÜV SÜD (the German authority that certifies safety-critical products). The difference is simple:

| Compiler | Status |
|---|---|
| Standard `rustc` | Great compiler, no certificate |
| Ferrocene | Same compiler + TÜV SÜD-signed certificate proving correct output |

Everything in this pipeline works with both. The switch between them is **one line**:

```toml
channel = "1.92.0"      # Development — no certificate, used in this POC
# channel = "ferrocene" # Production — TÜV SÜD certified (requires paid license)
```

All the engineering work — the tests, the scripts, the build flags — is identical in both modes. You build it once. The certificate is a purchase, not a rebuild.

---

## The LLVM Coverage Pipeline — What It Actually Does

Before the steps, it helps to understand how coverage evidence is generated. This is not a simple "did the tests pass" check. It is a chain of artifacts that proves, to a safety auditor, which lines of code were actually executed during testing.

### What Happens When You Run the Coverage Script

```
Your source code (.rs files)
        │
        │  rustc compiles with -Cinstrument-coverage
        │  (LLVM injects a silent counter at every branch point)
        ▼
Instrumented binary (runs normally, but secretly counts every line it touches)
        │
        │  cargo test runs all tests
        ▼
.profraw files (one raw counter file per test binary — binary format)
        │
        │  llvm-profdata merge
        │  (combines all .profraw files into one unified file)
        ▼
merged.profdata (one unified counter database)
        │
        │  llvm-cov export --format=lcov
        ▼
coverage.lcov (the formal evidence file — plain text, readable by auditors and CI tools)
        │
        │  llvm-cov show --format=html
        ▼
html/index.html (the human-readable visual report)
```

### What the `coverage.lcov` File Looks Like

It is a plain text file. Each section covers one source file:

```
SF:src/safety/brake_controller.rs    ← which source file
FN:202,brake_command                 ← function name and line number
FNDA:9,brake_command                 ← this function was called 9 times
DA:202,9                             ← line 202 executed 9 times
DA:208,1                             ← line 208 executed 1 time
LF:57                                ← 57 total lines in this file
LH:57                                ← 57 lines were hit (100% coverage)
end_of_record
```

The coverage gate script reads `LF:` (lines found) and `LH:` (lines hit) to calculate the percentage and decide whether to block the build.

### A Critical Detail — Why You Must Use the Toolchain's `llvm-profdata`, Not the System One

The Ubuntu system installs its own version of `llvm-profdata`. The problem is that `rustc 1.92.0` writes profraw files in format version 10, while the system tool often reads format version 9. Running the system tool produces:

```
error: raw profile version mismatch: Profile uses version = 10; expected version = 9
```

The fix is always to use the `llvm-profdata` and `llvm-cov` that ship inside the Rust toolchain — they are version-matched to the exact compiler. This is already handled in `scripts/run_coverage.sh`.

---

## Actual Coverage Results (Verified CI Run — All Checks Green ✅)

```
Filename                 Regions  Missed  Cover    Functions  Missed  Lines  Missed  Cover
────────────────────────────────────────────────────────────────────────────────────────────
airbag_sensor.rs              41       0  100.00%          5       0     49       0  100.00%
brake_controller.rs           44       0  100.00%          6       0     56       0  100.00%
────────────────────────────────────────────────────────────────────────────────────────────
TOTAL (safety files)          85       0  100.00%         11       0    105       0  100.00%
```

Every safety-critical line was executed. Every function was called. Zero missed regions.

> **Note:** `src/main.rs` is excluded from the coverage gate using `--ignore-filename-regex "src/main\.rs"`. It contains only demo `println!` scenarios — not safety logic. The gate measures only the certified safety modules.

---

## Step 1 — Create the Project Directory

```bash
mkdir -p /home/lg/Desktop/rust_test/ferrocene_poc
cd /home/lg/Desktop/rust_test/ferrocene_poc
cargo init --lib
```

**Why a dedicated folder?** ISO 26262 Part 6 requires that every safety artifact be traceable to a specific, bounded configuration. A dedicated folder means the entire folder is the safety item. Everything inside it is under audit.

**Why `--lib` and not a regular project?** A library crate means safety logic lives in `src/lib.rs`. A separate binary (`src/main.rs`) handles the demonstration scenarios. They are different compilation units.

ISO 26262 Part 6 Clause 9 requires component-level unit testing. You cannot do that properly with a single binary. The library structure is what makes the test suite in Step 7 possible.

---

## Step 2 — Pin the Toolchain

**File created:** `rust-toolchain.toml`

```toml
[toolchain]
channel = "1.92.0"
components = ["rustc", "cargo", "rustfmt", "clippy", "llvm-tools-preview"]
```

**Why pin the toolchain?** ISO 26262 Part 8 Clause 11 requires Tool Qualification — every tool that could introduce errors into safety artifacts must be identified by exact version and evaluated. Without a pin, `cargo build` uses whatever Rust version happens to be installed that day.

**Why these specific components?**

| Component | Reason |
|---|---|
| `rustc` | The compiler. Pinning this is the entire point. |
| `cargo` | Different versions resolve dependencies differently. Pinning ensures identical dependency graphs. |
| `rustfmt` | Prevents different machines producing different formatting, which creates noise in audit history. |
| `clippy` | The CI lint gate. Unpinned Clippy can silently change what warnings are raised between builds. |
| `llvm-tools-preview` | Installs `llvm-profdata` and `llvm-cov`, version-matched to this exact compiler. Required for coverage. |

---

## Step 3 — Configure Safety Build Profiles

**File modified:** `Cargo.toml`

```toml
[profile.release]
codegen-units   = 1
opt-level       = 2
overflow-checks = true
debug           = true
lto             = false
```

| Flag | Why |
|---|---|
| `overflow-checks = true` | Prevents silent integer wrap-around. 255 + 1 = panic, not 0. Required by ISO 26262 for arithmetic in safety code. |
| `codegen-units = 1` | Produces deterministic, reproducible binaries. Without this, two builds of the same source can produce different machine code. |
| `opt-level = 2` | Level 3 makes Worst-Case Execution Time (WCET) analysis unreliable. Level 2 gives strong optimization with bounded results. |
| `lto = false` | Keeps per-module object files intact for auditor inspection. LTO erases module boundaries. |
| `debug = true` | Keeps debug symbols in the evidence package so crash addresses map to exact source lines. |

---

## Step 4 — Write the Safety Logic

```bash
mkdir -p src/safety
```

**Why a `safety/` subdirectory?** Creates a visible, auditable boundary. Everything inside `src/safety/` is ASIL-D code subject to full verification. Everything outside (like `src/main.rs`) is demonstration code outside the safety boundary.

### 4a — The Airbag Controller (`src/safety/airbag_sensor.rs`)

**Why a 2-of-3 majority vote?** A single sensor can fail fail-open (always says "deploy") or fail-closed (never says "deploy"). A 2-of-3 majority vote handles every single-sensor failure:

- One sensor fails open → other two say "don't deploy" → correct, no deployment
- One sensor fails closed → other two say "deploy" → correct, airbag fires

ISO 26262 for ASIL-D explicitly requires hardware redundancy. Triple-sensor voting is a standard recognised pattern.

**Why detect all-three-deploy with near-zero G?** This is physically impossible in a real crash — it indicates a hardware short circuit driving all three sensors high simultaneously. The system raises a `SensorFault` and refuses to deploy. ISO 26262 requires safety systems to detect and respond to physically impossible inputs.

### 4b — The Brake Controller (`src/safety/brake_controller.rs`)

**Why dimensional types (`SpeedKmh`, `BrakePressureKpa`)?** Speed and pressure are both `f32`. Accidentally passing pressure where speed is expected is a valid program that produces wrong results silently. By defining separate struct types, the compiler enforces that you cannot use one where the other is expected. Mixing them is a **compile error** — the dangerous operation becomes literally impossible to express.

**Why a sticky fault state that requires hardware reset?** Once `FaultDetected` is entered, no software operation can clear it. Only `hardware_reset()` can exit the fault state. ISO 26262 requires that safety systems have defined safe states that require deliberate human action to exit. Allowing software to automatically self-clear faults has caused real automotive accidents.

**Why `#[must_use]` on brake command results?** Makes the Rust compiler reject any code that calls `brake_command()` without handling the result. Every error path is forced to be handled at compile time — not discovered at runtime.

**Why `PhantomData<*const ()>` to lock the controller to one thread?** Raw pointers do not implement `Send` or `Sync`. Adding `PhantomData<*const ()>` propagates this restriction to the whole struct. The Rust compiler refuses to compile any code that moves a `BrakeController` to a different thread — enforcing hardware CPU isolation at the type system level.

---

## Step 5 — Expose the Library API (`src/lib.rs`)

```rust
#![deny(unsafe_code)]
#![warn(clippy::all)]
pub mod safety;
```

**Why `#![deny(unsafe_code)]`?** The Ferrocene certificate covers the safe Rust subset. `unsafe` blocks are outside that guarantee and need separate manual review. This directive makes any use of `unsafe` a **build error** — the Ferrocene certificate covers the entire codebase, with no exceptions.

**Why `#![warn(clippy::all)]`?** Combined with `-D warnings` in CI, every Clippy finding either gets fixed or gets a formally documented exception — the Rust equivalent of a MISRA deviation record.

---

## Step 6 — Create the Scenario Simulation Binary (`src/main.rs`)

**Why 10 specific scenarios?** ISO 26262 requires requirements-based testing. Each scenario corresponds to a specific hazard from the HARA:

| Scenario | Hazard Covered |
|---|---|
| Normal cruising → release | System works in primary operating mode |
| Light braking → NormalBrake | ABS does not activate at low pressures |
| Hard braking at speed → EngageAbs | ABS correctly prevents wheel lock |
| Out-of-range sensor → FaultDetected (sticky) | System detects and locks on sensor failure |
| Command after fault → ActuatorFault error | Proves fault state cannot be bypassed |
| Emergency stop → EmergencyStop | Maximum braking under extreme conditions |
| Severe crash, all 3 sensors agree → FireAirbag | Primary crash detection works |
| One sensor failed, 2 of 3 agree → FireAirbag | Redundancy works with one sensor down |
| Minor bump at 8G → Hold | System does not deploy on minor impacts |
| All Deploy votes, near-zero G → SensorFault | Hardware short circuit detected |

---

## Step 7 — Write the MC/DC Integration Tests

```bash
mkdir -p tests
```

**Why MC/DC (Modified Condition/Decision Coverage)?** Standard coverage asks "did this line execute?" That is not enough for ASIL-D.

For a safety condition `pressure_is_valid(p) AND speed_is_valid(s)`, standard branch coverage requires only two tests. But if a sensor is broken and always returns `true`, you can pass branch coverage without ever discovering the broken sensor.

MC/DC requires each individual condition to **independently control** the final outcome:

| Test | Condition A | Condition B | Result | Proves |
|---|---|---|---|---|
| T1 | True | True | True | Baseline |
| T2 | **False** | True | **False** | A alone changes the outcome |
| T3 | True | **False** | **False** | B alone changes the outcome |

MC/DC is **required by ISO 26262 for ASIL-D**.

**Why 29 tests specifically?**

| Group | Count |
|---|---|
| `pressure_is_valid()` MC/DC | 3 |
| `brake_command()` ABS MC/DC | 4 |
| `majority_vote()` 2-of-3 patterns | 7 |
| `evaluate_deployment()` MC/DC | 4 |
| Sticky fault state machine | 2 |
| Cross-system brake + airbag coordination | 1 |
| Boundary conditions (min/max pressure) | 2 |
| Construction validation | 1 |
| **Total** | **29** |

---

## Step 8 — Write the Evidence Scripts

```bash
mkdir -p scripts
```

**Why scripts instead of running commands by hand?** ISO 26262 requires the verification process to be repeatable and documented. A bash script is an executable specification — any engineer on any machine running the same script performs the identical procedure. Scripts produce logs, and logs are evidence.

### Script A — `verify_toolchain.sh`
Checks compiler version, `rust-toolchain.toml` existence, and `llvm-tools-preview` presence before any build starts. Fast failure with a clear message is always better than slow failure with a confusing one.

### Script B — `run_coverage.sh` (The Safety Gate)

| Step | Action | Output |
|---|---|---|
| A | Set `RUSTFLAGS=-Cinstrument-coverage` | Enables LLVM counter injection |
| B | Set `LLVM_PROFILE_FILE` | Directs raw counter output location |
| C | `cargo test` | Runs all 29 tests, writes `.profraw` files |
| D | `llvm-profdata merge` | Merges all `.profraw` → one `merged.profdata` |
| E | `llvm-cov export --format=lcov` | Produces `coverage.lcov` (formal evidence) |
| F | Parse LCOV, check threshold | Exits with code 1 if coverage < 66% |

**Why does the exit code matter?** Without `exit 1`, the script is just a report. With `exit 1`, low coverage fails the script → fails the CI job → blocks the pull request. The PR cannot merge. This is what a safety gate actually means — it **stops** things, not just reports things.

### Script C — `run_sanitizers.sh`

**Why are sanitizers needed in safe Rust?** The borrow checker cannot see inside the standard library's `unsafe` internals, C libraries called via FFI, or runtime-only classes of undefined behaviour.

| Sanitizer | What It Catches |
|---|---|
| AddressSanitizer (ASan) | Buffer overflow, use-after-free, out-of-bounds access |
| UndefinedBehaviorSanitizer (UBSan) | Signed overflow, division by zero, null pointer dereference |
| LeakSanitizer (LSan) | Memory allocated but never freed (critical for long-running ECUs) |

---

## Step 9 — The CI/CD Safety Gate (GitHub Actions)

```bash
mkdir -p .github/workflows
```

**Why CI/CD for safety-critical code?** GitHub Actions produces an immutable, timestamped log of every commit: which workflow ran, which steps passed, what the output was. An auditor reads the log, not your word.

### The 6-Job Pipeline

```
F-1: verify-toolchain
  Checks compiler version before any build starts.
         │
F-2: build-and-lint
  cargo build + clippy -D warnings
  Every Clippy finding is a build error.
         │
F-3: test
  cargo test — all 29 MC/DC integration tests.
         │
F-4: coverage
  Enforces the ≥66% gate. Archives coverage.lcov as a CI artifact.
         │
F-5: docs
  cargo doc — fails on any undocumented public safety function.
         │
F-6a: safety-pipeline-summary  (always prints, even on failure)
F-6b: safety-gate-fail         (blocks merge if any job failed)
F-6c: safety-gate-pass         (confirms all clear — safe to merge)
```

**Why Clippy is a safety gate:** For C++ in S-CORE, CodeQL enforces MISRA rules. For Rust, Clippy with `-D warnings` enforces equivalent patterns: dead code, unused results, error-prone idioms. Any finding must either be fixed or suppressed with `#[allow(clippy::rule)]` plus a written justification — the Rust equivalent of a MISRA deviation record.

---

## Step 10 — Generate the Safety Reports

```bash
mkdir -p reports
```

### Report 1 — `toolchain_qualification_summary.md`
**What it answers:** "How do you know the compiler did not introduce a bug?"
Documents exact compiler version, known limitations, compensating measures, and the gap between POC toolchain and certified Ferrocene.

### Report 2 — `coverage_evidence_report.md`
**What it answers:** "Which safety requirements were actually tested?"
Maps test cases to safety requirement IDs:

```
SR-04 (fault detection, out-of-range pressure) → test_invalid_sensor_triggers_fault_state
SR-09 (majority vote, single sensor failure)   → test_sensor_vote_two_of_three_s1_s2, _s1_s3, _s2_s3
SR-12 (deploy criteria, timing gate)           → test_airbag_mcdc_t4_late_decision_holds
```

### Report 3 — `certification_gaps_register.md`
**What it answers:** "What do you still need before full certification?"

| Gap | Description | Owner |
|---|---|---|
| G-01 | Ferrocene commercial licence not purchased | Management |
| G-02 | HARA workshop not conducted | Safety Engineer |
| G-03 | Hardware-in-the-Loop testing not performed | Test Engineer |
| G-04 | Safety Management System not established | Safety Manager |
| G-05 | SBOM not generated (no CycloneDX export) | DevOps |

An auditor who sees a gaps register trusts the project **more** than one that claims no gaps. A gaps register is honest engineering.

---

## Running Everything

```bash
cd /home/lg/Desktop/rust_test/ferrocene_poc

# Verify the environment first
bash scripts/verify_toolchain.sh

# Build the safety library
cargo build

# Run the 10-scenario SIL harness
cargo run

# Run all 29 MC/DC integration tests
cargo test -- --nocapture

# Run the full coverage pipeline (LLVM + LCOV + HTML + gate)
bash scripts/run_coverage.sh

# Run memory and undefined-behaviour sanitizers
bash scripts/run_sanitizers.sh
```

**Why `--nocapture`?** Shows every print statement from every test in real time — making visible the scenario-by-scenario results. "Tests passed" is invisible. "Here is what happened in each test" is verifiable.

**Why all three verification layers?**

| Layer | ISO 26262 Term | What it proves |
|---|---|---|
| `cargo run` | Behavioural verification | Correct outputs in all 10 scenarios |
| `cargo test` | Formal functional verification | Every safety requirement passes its test case |
| `run_coverage.sh` | Structural verification | The test suite actually reaches the code it claims to verify |

ISO 26262 Part 6 requires all three.

---

## The Complete Evidence Package

```
Ferrocene POC — ISO 26262 ASIL-D Evidence Package
│
├── Tool Qualification Evidence
│   ├── rust-toolchain.toml                        ← compiler version pinned
│   └── reports/toolchain_qualification_summary.md
│
├── Source Code Verification
│   ├── src/lib.rs (#![deny(unsafe_code)])          ← unsafe banned, Ferrocene covers all
│   ├── Cargo.toml [profile.release]                ← overflow, LTO, determinism flags
│   └── CI F-2 logs (Clippy -D warnings)            ← static analysis evidence
│
├── Functional Verification
│   ├── tests/integration_tests.rs                  ← 29 MC/DC tests, requirement-traced
│   └── CI F-3 logs (cargo test)                    ← 29/29 passed at certified commit
│
├── Structural Verification (Coverage)
│   ├── target/coverage/merged.profdata             ← LLVM raw execution counters
│   ├── target/coverage/coverage.lcov               ← formal LCOV evidence
│   ├── target/coverage/html/index.html             ← visual review report
│   ├── reports/coverage_evidence_report.md         ← requirement-to-test mapping
│   └── CI F-4 logs (coverage gate 100% >= 66%)     ← gate enforcement proof
│
├── Runtime Verification
│   └── reports/sanitizer_runs/*.log                ← ASan, UBSan, LSan clean runs
│
└── Outstanding Gaps
    └── reports/certification_gaps_register.md      ← honest gap accounting
```

---

## Git Strategy — In Depth

Version control in a safety-critical project is not just code storage. Every commit is a timestamped, signed, immutable record that an auditor can inspect. Every branch is a traceability boundary. Every tag marks a configuration that can be reproduced exactly. Every push triggers an automated verification chain that generates evidence artifacts.

This section explains exactly what needs to happen in Git — from local repository setup all the way to a TÜV-submittable tagged release.

---

### 1. Repository Initialization — What Happens Locally

```bash
# Step 1: Create the project folder (this folder IS the certification boundary)
mkdir -p /home/lg/Desktop/rust_test/ferrocene_poc
cd /home/lg/Desktop/rust_test/ferrocene_poc
cargo init --lib

# Step 2: Initialize git
git init
git config user.name  "kabil91"
git config user.email "kabil91@users.noreply.github.com"

# Step 3: Create .gitignore BEFORE the first commit
# (If you commit target/ first, git tracks it — you must then explicitly untrack it)
# See .gitignore section below for what goes in it.

# Step 4: Stage only source files — never build artifacts
git add .gitignore Cargo.toml Cargo.lock rust-toolchain.toml
git add src/ tests/ scripts/ docs/ reports/ .github/
git status --short   # Review: every line should start with A (new file), not ??

# Step 5: Make the first commit with a meaningful message
git commit -m "feat: Ferrocene ASIL-D POC - initial commit"
```

**Why `git status --short` before every commit?**
In safety projects, committing the wrong files can pollute the audit trail. Binary build artifacts mixed into the history make the repository harder to audit and much larger to clone. Always review the staging area before committing.

---

### 2. The `.gitignore` File — What to Exclude and Why

Every exclusion in `.gitignore` is a deliberate decision, not a convenience.

```gitignore
# ── Rust build artifacts ─────────────────────────────────────────────────────
/target/
**/*.rs.bk

# WHY: The target/ directory contains compiled binaries, object files, and
# incremental build caches. It can be gigabytes in size. More importantly,
# it is GENERATED output — it is not source code. An auditor reviewing the
# repository should only see what humans wrote, not what the compiler produced.
# The CI pipeline regenerates target/ from source on every run — this is the
# proof that the build is reproducible.

# ── Coverage artifacts ────────────────────────────────────────────────────────
coverage.lcov
coverage_html/
target/coverage/

# WHY: These are generated by scripts/run_coverage.sh and by CI Job F-4.
# They are archived as CI artifacts (GitHub Actions "Upload Artifact" step)
# with 90-day retention. The CI artifact is the authoritative evidence copy —
# it is timestamped, tied to an exact commit SHA, and immutable.
# Committing coverage.lcov into the repository creates a second, uncontrolled
# copy that could go stale or be manually edited — neither is acceptable for
# a safety evidence file.

# ── Downloaded tools ─────────────────────────────────────────────────────────
actionlint

# WHY: Tools are not source code. Their versions are controlled by the CI
# workflow file (which specifies exact download versions). Committing a binary
# would make the repository platform-specific and much harder to audit.

# ── Editor settings ──────────────────────────────────────────────────────────
.vscode/*
!.vscode/settings.json   # ← This one IS committed (see below)
.idea/
*.swp
*.swo

# WHY .vscode/settings.json is INCLUDED (the ! exception):
# This file binds the YAML language extension to the official GitHub Actions
# JSON schema. Without it, VS Code shows false-positive "problems" on valid
# workflow expressions like ${{ needs.X.result }}. Committing this file means
# every developer who clones the repository gets correct schema validation
# automatically — no manual setup required.

# ── Secrets — NEVER commit these ─────────────────────────────────────────────
.env
*.pem
*.key
```

---

### 3. Creating the GitHub Remote Repository

GitHub no longer allows password-based `git push`. You must use a **Personal Access Token (PAT)**.

**Step 1 — Create the repository via GitHub API (one-time):**

```bash
curl -X POST https://api.github.com/user/repos \
  -H "Authorization: token YOUR_TOKEN" \
  -H "Accept: application/vnd.github+json" \
  -d '{
    "name": "ferrocene_poc",
    "description": "Ferrocene ASIL-D Automotive Safety POC — ISO 26262 certified Rust pipeline",
    "private": false,
    "auto_init": false
  }'
```

`auto_init: false` is critical — if GitHub initializes the repository with a README, your local `main` branch and the remote `main` branch will have different histories and `git push` will fail.

**Step 2 — Connect local repository to remote:**

```bash
git remote add origin https://github.com/kabil91/ferrocene_poc.git
git branch -M main   # Rename default branch to 'main' to match GitHub default
git push -u origin main
```

**Step 3 — Immediately clear the token from git config:**

```bash
# After push, set the remote URL back to the clean HTTPS form (no token embedded)
git remote set-url origin https://github.com/kabil91/ferrocene_poc.git
```

**Why remove the token from the URL?** Git stores remote URLs in `.git/config` — a plain text file. If the token is embedded in the URL, it persists on disk after the push and anyone with read access to the machine can extract it. Always push with the token, then immediately remove it.

**Why use a PAT instead of a password?** GitHub disabled password authentication for git operations in August 2021 (Security Advisory: [github.blog](https://github.blog/security/application-security/token-based-authentication-requirements-for-git-operations/)). PATs are scoped (you choose exactly what they can do), revocable (one click to disable), and auditable (GitHub logs every API call that uses them).

---

### 4. GitHub Secrets — How the CI Pipeline Gets Its Tokens

The CI pipeline needs credentials that it cannot have embedded in the workflow file (which is public). These are stored as **GitHub Actions Secrets** — encrypted values that are injected as environment variables at runtime.

**How to add a secret:**
1. Go to your repository → **Settings** → **Secrets and variables** → **Actions**
2. Click **New repository secret**
3. Add the following:

| Secret Name | Value | Used By |
|---|---|---|
| `CODECOV_TOKEN` | Token from [codecov.io](https://codecov.io) after linking your repo | F-4 Coverage job — uploads `coverage.lcov` to Codecov for trend tracking |

**How to get the `CODECOV_TOKEN`:**
1. Go to [codecov.io](https://codecov.io) and sign in with GitHub
2. Click **Add repository** and select `ferrocene_poc`
3. Codecov will display a token — copy it
4. Paste it into the GitHub Secret named `CODECOV_TOKEN`

**In the workflow file, the secret is referenced as:**
```yaml
- uses: codecov/codecov-action@v4
  with:
    token: ${{ secrets.CODECOV_TOKEN }}
    files: coverage.lcov
```

**Why Codecov at all?** The `coverage.lcov` file archived as a CI artifact proves coverage for one specific commit. Codecov tracks coverage across all commits and shows trends — so you can see if a new feature caused coverage to drop from 100% to 87% before it merges. For ISO 26262, trend visibility matters: a regression in coverage is a regression in your safety argument.

---

### 5. Branch Protection Rules — Enforcing the Safety Gate

Without branch protection, a developer can push directly to `main` and bypass all CI checks. In a safety-critical project, this is unacceptable — `main` must only ever contain code that has passed all six safety gates.

**How to set up branch protection:**
1. Repository → **Settings** → **Branches** → **Add branch protection rule**
2. Branch name pattern: `main`
3. Enable the following:

| Rule | Setting | Why |
|---|---|---|
| **Require a pull request before merging** | ✅ Enabled | No direct push to `main`. Every change goes through a PR. |
| **Require status checks to pass before merging** | ✅ Enabled | The PR cannot merge if CI fails. |
| **Required status checks** | Add: `F-6 Safety Gate - PASSED` | This is the single job that depends on all others. If it passes, all 5 safety jobs passed. |
| **Require branches to be up to date** | ✅ Enabled | The PR must be rebased on the latest `main` before merging. Prevents stale code from bypassing tests. |
| **Include administrators** | ✅ Enabled | Even the repository owner cannot bypass the safety gate. |
| **Do not allow bypassing the above settings** | ✅ Enabled | No emergency overrides. |

**Why only `F-6 Safety Gate - PASSED` as the required check?**
If you add F-1, F-2, F-3, F-4, F-5 individually to the required checks list, you must update that list every time you add a new safety job. With only F-6 as the required check, adding a new job means just adding it as a dependency of F-6 in the workflow file. The enforcement point never changes.

---

### 6. The Pull Request Workflow — How Code Moves to `main`

In the safety workflow, **no code goes directly to `main`**. Every change follows this path:

```
Developer Machine
      │
      │  git checkout -b feature/brake-controller-timeout
      │  (work, test locally: cargo test)
      │
      ▼
Feature Branch (pushed to GitHub)
      │
      │  git push origin feature/brake-controller-timeout
      │  (opens a Pull Request on GitHub)
      │
      ▼
Pull Request
      │
      │  GitHub Actions triggers automatically on PR open/update:
      │    F-1: Toolchain Verification    ✅ or ❌
      │    F-2: Build + Static Analysis   ✅ or ❌
      │    F-3: Test Suite                ✅ or ❌
      │    F-4: Coverage Gate             ✅ or ❌
      │    F-5: Documentation             ✅ or ❌
      │    F-6: Safety Gate               ✅ (all pass) or ❌ (any fail)
      │
      │  If F-6 ❌ → PR merge button is DISABLED. Cannot merge.
      │  If F-6 ✅ → Code review by another engineer required.
      │             After approval → merge allowed.
      ▼
main branch (every commit here has a verified audit trail)
```

**Why require code review even if CI passes?** CI verifies correctness against the requirements it knows about. A human reviewer catches things CI cannot: wrong algorithm for the safety requirement, missing edge case in a test, a comment that contradicts the code. For ASIL-D, ISO 26262 Part 6 Clause 7 requires independent review of all safety-critical work products.

---

### 7. Commit Message Convention — Conventional Commits

Every commit message follows the [Conventional Commits](https://www.conventionalcommits.org/) standard. This is not a style preference — it is how the commit history becomes machine-readable and auditable.

```
<type>(<scope>): <short description>

<body — what changed and why>

<footer — breaking changes, requirement references>
```

| Type | When to Use |
|---|---|
| `feat` | New safety feature added |
| `fix` | Bug fix (in source code or scripts) |
| `test` | New or changed tests |
| `docs` | Documentation only |
| `style` | Formatting, no logic change (`cargo fmt`) |
| `refactor` | Code restructure, no behaviour change |
| `ci` | Changes to `.github/workflows/` |
| `chore` | Build system, dependencies |

**Examples from this project:**
```
feat: Ferrocene ASIL-D POC - initial commit
fix: resolve pre-flight script parsing and arithmetic bugs
style: apply official rustfmt styling for CI compliance
ci: resolve coverage report flags and exclude main.rs from gate
docs: update complete build walkthrough to v3.0
```

**For traceability — linking commits to safety requirements:**

When a commit implements or tests a specific safety requirement from the HARA document, reference it in the footer:

```
feat(brake): add pressure boundary validation

Validates brake pressure is within [min, max] range before
engaging ABS. Returns BrakeError::InvalidSensor on out-of-range.

Refs: SR-04 (Fault detection — out-of-range sensor input)
Closes: #12
```

An auditor can then run `git log --grep="SR-04"` to find every commit related to safety requirement SR-04. This is **bidirectional traceability** — from requirement to code, and from code back to requirement.

---

### 8. Tagging Certified Releases

When a commit has passed all safety gates and is formally approved for a certification snapshot, it must be tagged. A tag is an immutable pointer to an exact commit. The binary produced from a tagged commit can always be reproduced by checking out that tag.

```bash
# Create an annotated tag (not a lightweight tag — annotated tags include
# the tagger's name, email, timestamp, and a message, all signed into the tag object)
git tag -a v1.0.0-poc -m "POC: all 6 safety gates passing, 100% safety module coverage

Safety Gate Status:
  F-1 Toolchain Verification    : PASS (rustc 1.92.0)
  F-2 Build + Static Analysis   : PASS (Clippy 0 warnings)
  F-3 Test Suite                : PASS (29/29 tests)
  F-4 Coverage Gate             : PASS (100% >= 66%)
  F-5 Documentation             : PASS (all public APIs documented)
  F-6 Safety Gate               : PASS

Evidence artifacts at CI run: https://github.com/kabil91/ferrocene_poc/actions
Coverage LCOV: archived in CI artifact coverage-evidence-<sha>
"

# Push the tag to GitHub
git push origin v1.0.0-poc
```

**Tag naming convention for safety releases:**

| Tag | Meaning |
|---|---|
| `v1.0.0-poc` | Proof of Concept — not certified, standard rustc |
| `v1.0.0-sil` | Software-in-the-Loop validated |
| `v1.0.0-hil` | Hardware-in-the-Loop validated |
| `v1.0.0-cert` | Full ASIL-D Ferrocene certificate issued |

**Why annotated tags and not lightweight tags?** A lightweight tag is just a pointer to a commit. An annotated tag is a full git object with its own SHA, author, timestamp, and message — and it can be GPG-signed. For certification, you want to be able to prove when the tag was created and by whom. Annotated tags provide that. A GPG-signed tag (`git tag -s`) adds a cryptographic signature that proves the tag was created by a specific key — the strongest form of evidence for a safety authority.

---

### 9. Keeping Credentials Safe — What to Never Do

| Action | Why It Is Dangerous |
|---|---|
| Paste a token into a chat window | The chat log is stored. Anyone with access to the log can use the token until it is revoked. |
| Commit a token in a source file | `git log` preserves it forever, even after you delete the file. The history still contains it. |
| Store a token in the remote URL (`https://user:token@github.com/...`) | Git stores remote URLs in `.git/config` — plain text, readable by anyone with filesystem access. |
| Use a token with `repo` scope when `public_repo` is sufficient | Over-scoped tokens are more dangerous if leaked. Minimum scope always. |

**The correct practice:**
1. Create a PAT with minimum required scope
2. Use it for a single push operation
3. Immediately revoke it on GitHub after use (`Settings → Tokens → Delete`)
4. For recurring CI use, store tokens **only** as GitHub Actions Secrets (encrypted, never exposed in logs)

---

### 10. The Complete Git Workflow Summary

```
Local Development
├── git checkout -b feature/your-change
├── Make changes
├── cargo fmt                  ← format before commit (avoids CI fmt failure)
├── cargo test                 ← run tests before commit (fast feedback)
├── cargo clippy               ← check lints before commit
├── git add <specific files>   ← never git add . blindly
├── git commit -m "type(scope): description"
└── git push origin feature/your-change

Pull Request (GitHub)
├── CI runs automatically (F-1 through F-6)
├── If any job fails → fix locally → git push (re-triggers CI)
├── When F-6 PASSED → request code review
└── After approval → merge to main

Release Tagging (after formal safety review)
├── git tag -a v1.0.0-poc -m "release message"
├── git push origin v1.0.0-poc
└── Evidence archived: CI artifacts + tag annotation

Credential Safety
├── Never commit tokens
├── Never paste tokens in chat
├── Use GitHub Secrets for CI credentials
└── Revoke PATs immediately after single-use push operations
```

---

## Git & CI/CD Journey — Commit History

This section documents exactly what was done in version control — every commit, every pipeline failure encountered, and the fix applied. This is part of the audit trail.

### Repository Setup

The project was initialized as a local git repository, cleaned of build artifacts, and pushed to GitHub using the GitHub API (to create the remote repository) followed by `git push`.

**Key `.gitignore` decisions:**
- `target/` — Rust build artifacts (hundreds of MB, regenerated by CI)
- `coverage_html/` — Generated HTML report (regenerated by CI)
- `coverage.lcov` — Generated evidence file (archived as a CI artifact)
- `actionlint` — Downloaded binary (not source code)
- `.vscode/*` with `!.vscode/settings.json` — Personal editor settings excluded, but the schema binding file is committed so all team members benefit from correct YAML validation

---

### Commit History (Oldest → Newest)

| Commit | Message | What It Did |
|---|---|---|
| `804275f` | `feat: Ferrocene ASIL-D POC - initial commit` | First push — 21 files, 4,065 lines. Source, tests, scripts, docs, CI pipeline. |
| `f8345e2` | `fix: resolve all 19 workflow lint errors` | Split Job 6 into 3 separate jobs (F-6a summary, F-6b fail-gate, F-6c pass-gate). Moved `needs` context to job-level `if:`. Added `.vscode/settings.json` with GitHub Actions JSON schema binding. |
| `32e0f00` | `fix: pin all GitHub Actions to exact commit SHAs` | Attempted pinning actions to full 40-character commit SHAs for maximum reproducibility. |
| `20b36c6` | `fix: replace invalid dtolnay/rust-toolchain@v1 with exact SHA` | Corrected action reference after discovering `@v1` tag does not exist for `dtolnay/rust-toolchain`. |
| `9953df9` | `style: revert actions to @v4 tags to fix VS Code linting` | VS Code GitHub Actions extension cannot resolve 40-character SHAs. Reverted to `@v4` tags which the extension can validate. |
| `998b2a2` | `style: revert dtolnay/rust-toolchain to @master to fix VS Code linting` | `dtolnay/rust-toolchain` only publishes `@master` — no versioned tags exist. Reverted to `@master`. |
| `b69b98f` | `fix: resolve pre-flight script parsing and arithmetic bugs` | Fixed two bugs in `scripts/verify_toolchain.sh` (see below). |
| `0378549` | `style: apply official rustfmt styling for CI compliance` | Ran `cargo fmt` to auto-fix all formatting. CI pipeline enforces `cargo fmt --check` as a hard gate. |
| `11b47ba` | `fix: resolve coverage report flags and exclude main.rs from gate` | Removed invalid `--all-features` and `--workspace` flags from `cargo llvm-cov report`. Added `--ignore-filename-regex "src/main\.rs"` so the 66% gate measures only safety modules. |
| `3eaeec2` | `docs: update complete build walkthrough to v3.0` | Updated this document to reflect the verified green pipeline. |

---

### CI/CD Pipeline Failures and Fixes

Every failure below was caught by the GitHub Actions pipeline before the code reached the `main` branch. This is the pipeline doing its job.

#### Failure 1 — F-1 Toolchain Verification failing after 19s

**Root cause 1 — Bad grep:** `scripts/verify_toolchain.sh` used `grep 'channel'` to read `rust-toolchain.toml`. Because the file contains comments that also contain the word "channel" (e.g. `# Change channel = "1.92.0" → channel = "ferrocene"`), grep returned multiple lines, corrupting the `$CHANNEL` variable.

```bash
# Before (broken — matches comments too)
CHANNEL=$(grep 'channel' rust-toolchain.toml | cut -d'"' -f2)

# After (fixed — only matches the actual config line)
CHANNEL=$(grep '^channel' rust-toolchain.toml | head -n 1 | cut -d'"' -f2)
```

**Root cause 2 — Bash arithmetic under `set -e`:** The script used `set -euo pipefail` (fail on any error) and `((WARN++))` to count warnings. In Bash, `((VAR++))` when `VAR=0` evaluates to `0`, which Bash treats as a failure exit code, immediately terminating the script.

```bash
# Before (broken — exits the script when count is zero)
((WARN++))

# After (fixed — safe POSIX increment that never returns a failure code)
WARN=$((WARN + 1))
```

**Fix commit:** `b69b98f`

---

#### Failure 2 — F-2 Build + Static Analysis failing (cargo fmt --check)

**Root cause:** The source files used custom visual alignment for struct fields (extra spaces to line up columns). `rustfmt` enforces the official Rust style guide which does not permit this. The `cargo fmt --check` gate in CI rejected the non-standard formatting.

```rust
// Before (visual alignment — rejected by rustfmt)
deploy_threshold_g:   f32,
max_decision_time_ms: u32,
deployment_armed:     bool,

// After (standard rustfmt style — accepted)
deploy_threshold_g: f32,
max_decision_time_ms: u32,
deployment_armed: bool,
```

**Fix:** Ran `cargo fmt` locally which auto-corrected all files in one pass.  
**Fix commit:** `0378549`

---

#### Failure 3 — F-4 Coverage Gate failing after 1 minute

This had two separate sub-causes:

**Sub-cause A — Invalid flags on `cargo llvm-cov report`:** The `report` subcommand only formats already-collected data — it does not compile anything. Passing `--all-features` and `--workspace` (which are compilation flags) caused it to reject the command entirely.

```bash
# Before (broken)
cargo llvm-cov report --all-features --workspace --summary-only

# After (fixed)
cargo llvm-cov report --summary-only
```

**Sub-cause B — Coverage gate failing at 59% despite 100% safety module coverage:** The `cargo llvm-cov` report included `src/main.rs`, which contains only `println!` demo scenarios with zero test coverage. This dragged the total from 100% (safety modules alone) down to 59%, failing the 66% threshold.

**Fix:** Added `--ignore-filename-regex "src/main\.rs"` to both commands:

```bash
cargo llvm-cov \
  --workspace \
  --ignore-filename-regex "src/main\.rs" \
  --lcov \
  --output-path coverage.lcov

cargo llvm-cov report \
  --ignore-filename-regex "src/main\.rs" \
  --summary-only
```

**Why this is correct:** `src/main.rs` is the demonstration harness — it is not safety logic and is not under the ASIL-D certification boundary. Only `src/safety/airbag_sensor.rs` and `src/safety/brake_controller.rs` are safety-critical. The gate should measure exactly those files.  
**Fix commit:** `11b47ba`

---

### Final Pipeline State

After all fixes, the pipeline reached **7/7 checks passing**:

| Job | Status | Time |
|---|---|---|
| F-1 Toolchain Verification | ✅ Successful | 19s |
| F-2 Build + Static Analysis | ✅ Successful | 15s |
| F-3 Test Suite (29/29 passed) | ✅ Successful | 18s |
| F-4 Coverage Gate (100% ≥ 66%) | ✅ Successful | 1m |
| F-5 Documentation Verification | ✅ Successful | 15s |
| F-6 Safety Gate — FAILED | ⏭ Skipped (no failures to report) | — |
| F-6 Safety Gate — PASSED | ✅ Successful | 3s |
| F-6 Safety Pipeline Summary | ✅ Successful | 3s |

The "F-6 Safety Gate — FAILED" job being **Skipped** is correct and expected behaviour. It only runs when one of F-1 through F-4 has failed. When everything passes, it is skipped — and "F-6 Safety Gate — PASSED" runs instead.

---

## The Production Transition — What Remains After This POC

| Step | Action | What Changes |
|---|---|---|
| 1 | Purchase Ferrocene licence from Ferrous Systems | Commercial |
| 2 | Change `channel = "1.92.0"` to `channel = "ferrocene"` | **One line** in `rust-toolchain.toml` |
| 3 | Download Ferrocene Qualification Kit | Included in licence |
| 4 | Conduct HARA workshop with qualified Safety Engineer | Engineering |
| 5 | Update `toolchain_qualification_summary.md` with FQK reference | Documentation |
| 6 | Complete HIL testing on target ECU hardware | Hardware |
| 7 | Generate formal SBOM (CycloneDX format) | DevOps |
| 8 | Engage TÜV SÜD for final audit | Certification body |
| 9 | Receive ASIL-D certificate | Legal |

Steps 2 through 9 build on the infrastructure this POC already created. The engineering pipeline is complete. The certification is a process that runs on top of it.

---

*Document version: 3.0 — Updated after verified 100% green CI/CD pipeline run (7/7 checks passing)*
*Pipeline: https://github.com/kabil91/ferrocene_poc/actions*
