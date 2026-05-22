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
