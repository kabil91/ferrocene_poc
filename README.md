# Ferrocene ASIL-D POC
## Automotive Safety Controller — From First Principles to Safety Evidence

> **Self-contained POC** demonstrating the complete Ferrocene safety pipeline:  
> source code → compile → test → MC/DC coverage → LCOV evidence → CI enforcement.  
> Every file contains numbered **MANUAL STEP** comments explaining exactly what  
> to do, why it matters, and what changes for full ASIL-D certification.

---

## Quick Start

```bash
# 1. Clone / open the project
cd /home/lg/Desktop/rust_test/ferrocene_poc

# 2. Verify toolchain (pre-flight check)
bash scripts/verify_toolchain.sh

# 3. Build the project
cargo build

# 4. Run the safety scenario demo
cargo run

# 5. Run all tests (unit + integration + MC/DC)
cargo test -- --nocapture

# 6. Run full coverage pipeline (with threshold gate)
bash scripts/run_coverage.sh

# 7. Run sanitizers
bash scripts/run_sanitizers.sh
```

---

## Table of Contents

| Section | Topic |
|---------|-------|
| [1. What This POC Demonstrates](#1-what-this-poc-demonstrates) | Goals and scope |
| [Detailed Theory (Simple Language)](docs/ferrocene_theory.md) | **Start Here: Simple Theory** |
| [Manual Steps Walkthrough](docs/manual_steps_walkthrough.md) | **Step-by-Step Theory** |
| [2. Project Structure](#2-project-structure) | All files explained |
| [3. Manual Steps Index](#3-manual-steps-index) | All 60 steps at a glance |
| [4. The One-Line Ferrocene Switch](#4-the-one-line-ferrocene-switch) | How to go certified |
| [5. Safety Architecture](#5-safety-architecture) | Brake + airbag design |
| [6. MC/DC Coverage Guide](#6-mcdc-coverage-guide) | What gets tested and why |
| [7. Coverage Pipeline](#7-coverage-pipeline) | .profraw → LCOV walkthrough |
| [8. CI/CD Pipeline](#8-cicd-pipeline) | All 6 safety gate jobs |
| [9. Certification Gaps](#9-certification-gaps) | What is left for TÜV |
| [10. Ferrocene vs Standard rustc](#10-ferrocene-vs-standard-rustc) | Comparison table |
| [11. Quick Reference](#11-quick-reference) | All commands in one place |

---

## 1. What This POC Demonstrates

This project is a **running, testable, coverable** proof-of-concept for the Ferrocene  
ASIL-D safety pipeline described in the Ferrocene document (F-1 through F-18).

### What It Shows

| Demonstrated Concept | Where in This POC |
|----------------------|-------------------|
| Compile-time memory safety | All `.rs` files — borrow checker enforced |
| Newtype pattern (dimensional safety) | `BrakePressureKpa`, `SpeedKmh`, `AccelerationG` |
| Explicit error handling | `Result<BrakeAction, BrakeError>` with `#[must_use]` |
| Fail-safe state machine | `BrakeState` enum, `FaultDetected` sticky state |
| Sensor diversity (2-of-3 voting) | `SensorVote::majority_vote()` |
| MC/DC test coverage | `tests/integration_tests.rs` — T1/T2/T3 sets |
| Coverage threshold gate | `scripts/run_coverage.sh` — exits 1 if below 66% |
| Sanitizer evidence collection | `scripts/run_sanitizers.sh` |
| CI/CD enforcement | `.github/workflows/ferrocene_pipeline.yml` |
| Toolchain pinning | `rust-toolchain.toml` — one file, zero drift |
| ISO 26262 traceability | `REQ-BRAKE-001` comments in source |

### What This POC Does NOT Include

| Not Included | Why / Where To Find It |
|-------------|------------------------|
| Actual Ferrocene binary | Requires commercial license from Ferrous Systems |
| Bazel build system | Requires S-CORE workspace (`/home/lg/Desktop/score`) |
| CodeQL/MISRA analysis | Separate concern — documented in MISRA pipeline |
| HARA document | Requires safety engineering workshop |
| TÜV SÜD certificate | Requires €50k–200k TÜV engagement |
| QNX cross-compilation | Requires QNX SDK and Ferrocene QNX target |

---

## 2. Project Structure

```
ferrocene_poc/
│
├── rust-toolchain.toml              ← MANUAL STEP 1-2: The one-line Ferrocene switch
├── Cargo.toml                       ← MANUAL STEP 3-5: Safety build profiles
│
├── src/
│   ├── lib.rs                       ← MANUAL STEP 17: Library root + crate-level lints
│   ├── main.rs                      ← MANUAL STEP 18-19: Demo binary (10 scenarios)
│   └── safety/
│       ├── mod.rs                   ← MANUAL STEP 15-16: Module visibility
│       ├── brake_controller.rs      ← MANUAL STEP 6-14: ASIL-D brake logic + MC/DC
│       └── airbag_sensor.rs         ← MANUAL STEP 11-14: Airbag sensor fusion
│
├── tests/
│   └── integration_tests.rs         ← MANUAL STEP 20-26: Full MC/DC test suites
│
├── scripts/
│   ├── run_coverage.sh              ← MANUAL STEP 27-32: .profraw → LCOV pipeline
│   ├── run_sanitizers.sh            ← MANUAL STEP 33-37: ASan/UBSan/LSan evidence
│   └── verify_toolchain.sh          ← MANUAL STEP 38: Pre-flight toolchain checks
│
├── .github/
│   └── workflows/
│       └── ferrocene_pipeline.yml   ← MANUAL STEP 39-50: 6-job CI safety pipeline
│
└── reports/
    ├── toolchain_qualification_summary.md  ← MANUAL STEP 51-53: ISO 26262 Part 8 record
    ├── coverage_evidence_report.md         ← MANUAL STEP 54-57: Coverage tracking
    └── certification_gaps_register.md      ← MANUAL STEP 58-60: What is left for TÜV
```

---

## 3. Manual Steps Index

Every file in this project contains numbered **MANUAL STEP** comments.  
Here is the complete index:

| Step | Location | Topic |
|------|----------|-------|
| 1  | `rust-toolchain.toml` | Current state — standard rustc for POC |
| 2  | `rust-toolchain.toml` | Production switch — `channel = "ferrocene"` |
| 3  | `Cargo.toml` | Understanding the manifest |
| 4  | `Cargo.toml` | Adding sanitizer features |
| 5  | `Cargo.toml` | Running coverage profile |
| 6  | `brake_controller.rs` | Compile-time safety guarantees |
| 7  | `brake_controller.rs` | Constructor — explicit initialization |
| 8  | `brake_controller.rs` | MC/DC test design for pressure_is_valid() |
| 9  | `brake_controller.rs` | Result<T,E> — forced error handling |
| 10 | `brake_controller.rs` | Irreversible state transitions |
| 11 | `airbag_sensor.rs` | Sensor fusion pattern |
| 12 | `airbag_sensor.rs` | Thread safety via type system |
| 13 | `airbag_sensor.rs` | 2-of-3 majority voting |
| 14 | `airbag_sensor.rs` | Multi-condition MC/DC |
| 15 | `safety/mod.rs` | Module visibility rules |
| 16 | `safety/mod.rs` | Module structure rationale |
| 17 | `lib.rs` | Library vs binary split |
| 18 | `main.rs` | Running the POC |
| 19 | `main.rs` | Observing compile-time safety |
| 20 | `integration_tests.rs` | Why tests live in tests/ |
| 21 | `integration_tests.rs` | MC/DC — what tests prove |
| 22 | `integration_tests.rs` | Running the tests |
| 23 | `integration_tests.rs` | Reading the MC/DC matrix |
| 24 | `integration_tests.rs` | Testing state machine transitions |
| 25 | `integration_tests.rs` | 2-of-3 voting MC/DC |
| 26 | `integration_tests.rs` | Cross-module integration test |
| 27 | `run_coverage.sh` | Coverage pipeline step-by-step |
| 28 | `run_coverage.sh` | Ferrocene vs standard toolchain for coverage |
| 29 | `run_coverage.sh` | RUSTFLAGS explained |
| 30 | `run_coverage.sh` | Why merge .profraw files? |
| 31 | `run_coverage.sh` | llvm-cov and LCOV format |
| 32 | `run_coverage.sh` | Coverage gate — blocks CI merge |
| 33 | `run_sanitizers.sh` | What sanitizers catch |
| 34 | `run_sanitizers.sh` | Why same toolchain for sanitizers |
| 35 | `run_sanitizers.sh` | UBSan |
| 36 | `run_sanitizers.sh` | ASan |
| 37 | `run_sanitizers.sh` | LSan |
| 38 | `verify_toolchain.sh` | Pre-flight checklist |
| 39 | `ferrocene_pipeline.yml` | CI/CD auditor question |
| 40 | `ferrocene_pipeline.yml` | Ferrocene production vs POC |
| 41 | `ferrocene_pipeline.yml` | Job dependency graph |
| 42 | `ferrocene_pipeline.yml` | dtolnay/rust-toolchain action |
| 43 | `ferrocene_pipeline.yml` | cargo check vs cargo build |
| 44 | `ferrocene_pipeline.yml` | Clippy as MISRA-equivalent |
| 45 | `ferrocene_pipeline.yml` | rustfmt as safety evidence |
| 46 | `ferrocene_pipeline.yml` | What gets tested |
| 47 | `ferrocene_pipeline.yml` | LCOV artifact generation |
| 48 | `ferrocene_pipeline.yml` | The coverage gate |
| 49 | `ferrocene_pipeline.yml` | Archiving LCOV as CI artifact |
| 50 | `ferrocene_pipeline.yml` | Why doc tests matter |
| 51 | `toolchain_qualification_summary.md` | ISO 26262 Part 8 record |
| 52 | `toolchain_qualification_summary.md` | POC vs production |
| 53 | `toolchain_qualification_summary.md` | Platform qualification |
| 54 | `coverage_evidence_report.md` | Coverage run tracking |
| 55 | `coverage_evidence_report.md` | Why 66%? |
| 56 | `coverage_evidence_report.md` | Populating the run log |
| 57 | `coverage_evidence_report.md` | Coverage tool qualification |
| 58 | `certification_gaps_register.md` | Gap register overview |
| 59 | `certification_gaps_register.md` | GAP-01: Ferrocene license |
| 60 | `certification_gaps_register.md` | What is already complete |

---

## 4. The One-Line Ferrocene Switch

> This is the most important concept in the entire document.  
> The technical pipeline is IDENTICAL for standard rustc and Ferrocene.  
> The ONLY difference is one line in `rust-toolchain.toml`.

```toml
# CURRENT (POC — not certified):
channel = "1.92.0"

# PRODUCTION (ASIL-D certified):
channel = "ferrocene"
```

After this change:
- `cargo build` → compiles with TÜV-certified binary
- `cargo test` → tests compiled with certified binary
- `bash scripts/run_coverage.sh` → LCOV is now trusted safety evidence
- `bash scripts/run_sanitizers.sh` → sanitizer logs are valid safety evidence

**Zero code changes. Zero pipeline changes. One line.**

---

## 5. Safety Architecture

### Brake-by-Wire Controller (`src/safety/brake_controller.rs`)

```
BrakeController::new(max, min)
         │
         ▼
BrakeController::brake_command(speed, pressure)
         │
         ├── pressure_is_valid()?  ──NO──► FaultDetected state
         │         │                       (sticky — hardware_reset() needed)
         │        YES
         │         │
         │    speed > 20  AND  pressure > 50?
         │         │
         │        YES ──► EngageAbs { pressure }
         │         │
         │        NO  ──► NormalBrake { pressure }   (if pressure > 0)
         │                Release                     (if pressure = 0)
         │
         └── emergency_stop() ──► EmergencyStop state (one-way)
```

### Airbag Sensor Fusion (`src/safety/airbag_sensor.rs`)

```
SensorVote::majority_vote(s1, s2, s3)   ← 2-of-3 threshold
         │
         ▼
AirbagController::evaluate_deployment(votes, accel, time)
         │
         ├── disarmed? ──YES──► Hold
         │
         ├── all-deploy + near-zero-G? ──► SensorFault (impossible combination)
         │
         ├── majority=Deploy AND accel>=25G AND time<=30ms?
         │         │
         │        YES ──► FireAirbag { peak_acceleration, time_to_decision }
         │         │
         │        NO  ──► Hold
```

---

## 6. MC/DC Coverage Guide

### What MC/DC Is

For a boolean decision with N conditions:
- **Statement coverage**: Did the line execute? (N tests)
- **Branch coverage**: Did both true/false paths execute? (2N tests)
- **MC/DC**: Does each condition independently control the outcome? (N+1 tests minimum)

### MC/DC Test Sets in This POC

#### `pressure_is_valid()` — 2 conditions

| Test | Condition A (≥min) | Condition B (≤max) | Result | Proves |
|------|--------------------|--------------------|--------|--------|
| T1   | ✅ true (200≥1)    | ✅ true (200≤400)  | VALID  | Baseline |
| T2   | ❌ false (5<10)    | ✅ true (5≤400)    | INVALID| A independently controls |
| T3   | ✅ true (999≥1)    | ❌ false (999>400) | INVALID| B independently controls |

#### `brake_command()` ABS decision — 2 conditions

| Test | Condition A (speed>20) | Condition B (pressure>50) | ABS? | Proves |
|------|------------------------|---------------------------|------|--------|
| T1   | ✅ 100 km/h            | ✅ 200 kPa                | YES  | Baseline |
| T2   | ❌ 10 km/h             | ✅ 200 kPa                | NO   | Speed controls ABS |
| T3   | ✅ 100 km/h            | ❌ 30 kPa                 | NO   | Pressure controls ABS |

#### `evaluate_deployment()` — 3 conditions

| Test | A (majority) | B (accel≥25G) | C (time≤30ms) | Result | Proves |
|------|-------------|---------------|---------------|--------|--------|
| T1   | ✅ 2/3 vote | ✅ 30G        | ✅ 20ms       | FIRE   | Baseline |
| T2   | ❌ 1/3 vote | ✅ 30G        | ✅ 20ms       | HOLD   | Vote controls |
| T3   | ✅ 2/3 vote | ❌ 15G        | ✅ 20ms       | HOLD   | G controls |
| T4   | ✅ 2/3 vote | ✅ 30G        | ❌ 50ms       | HOLD   | Timing controls |

---

## 7. Coverage Pipeline

```
Developer runs: bash scripts/run_coverage.sh
                           │
    ┌──────────────────────▼──────────────────────┐
    │ Step A: RUSTFLAGS=-Cinstrument-coverage      │
    │         -Clink-dead-code -Ccodegen-units=1   │
    └──────────────────────┬──────────────────────┘
                           │
    ┌──────────────────────▼──────────────────────┐
    │ Step B/C: cargo test                         │
    │  → test1.profraw (brake tests)               │
    │  → test2.profraw (airbag tests)              │
    │  → test3.profraw (integration tests)         │
    └──────────────────────┬──────────────────────┘
                           │
    ┌──────────────────────▼──────────────────────┐
    │ Step D: llvm-profdata merge                  │
    │  → merged.profdata                           │
    └──────────────────────┬──────────────────────┘
                           │
    ┌──────────────────────▼──────────────────────┐
    │ Step E: llvm-cov export --format=lcov        │
    │  → coverage.lcov (TÜV-accepted format)       │
    └──────────────────────┬──────────────────────┘
                           │
    ┌──────────────────────▼──────────────────────┐
    │ Step F: Line coverage >= 66%?                │
    │  YES → Pipeline PASSES, LCOV archived        │
    │  NO  → Pipeline FAILS, PR blocked            │
    └─────────────────────────────────────────────┘
```

---

## 8. CI/CD Pipeline

The `.github/workflows/ferrocene_pipeline.yml` runs these 6 jobs on every PR:

```
┌──────────────────────────────────────────────────────┐
│  F-1: Toolchain Verification                         │
│  verify_toolchain.sh → rustc version check          │
└──────────────────┬───────────────────────────────────┘
                   │
┌──────────────────▼───────────────────────────────────┐
│  F-2: Build + Static Analysis                        │
│  cargo check → cargo build → clippy → rustfmt       │
└──────────────────┬───────────────────────────────────┘
                   │
┌──────────────────▼───────────────────────────────────┐
│  F-3: Test Suite                                     │
│  cargo test --no-fail-fast                           │
└──────────────────┬───────────────────────────────────┘
                   │
┌──────────────────▼───────────────────────────────────┐
│  F-4: Coverage Gate (≥66%)                          │
│  cargo llvm-cov → coverage.lcov → threshold check   │
│  BLOCKS MERGE if below threshold                     │
└──────────────────┬───────────────────────────────────┘
                   │
┌──────────────────▼───────────────────────────────────┐
│  F-5: Documentation Verification                     │
│  cargo doc → RUSTDOCFLAGS="-D warnings"              │
└──────────────────┬───────────────────────────────────┘
                   │
┌──────────────────▼───────────────────────────────────┐
│  F-6: Safety Pipeline Gate                           │
│  Aggregates all results → PASS or BLOCK MERGE        │
└──────────────────────────────────────────────────────┘
```

---

## 9. Certification Gaps

See `reports/certification_gaps_register.md` for full details.

| Gap | What | Effort |
|-----|------|--------|
| GAP-01 | Purchase Ferrocene license | Low (commercial) |
| GAP-02 | Assign Safety Manager | Medium (organizational) |
| GAP-03 | Write HARA | High (2-4 weeks) |
| GAP-04 | Write FSC + TSC | High |
| GAP-05 | Write Safety Plan | Medium |
| GAP-06 | V-Model lifecycle tool | Medium |

> The **code** and **pipeline** are complete. All gaps are commercial or organizational.

---

## 10. Ferrocene vs Standard rustc

| Property | Standard rustc (this POC) | Ferrocene (production) |
|----------|--------------------------|------------------------|
| Binary output | Identical to Ferrocene | Identical to rustc |
| Compiler source | Upstream rolling | LTS snapshot — frozen |
| Updates | Every 6 weeks | Never (CVEs backported only) |
| Language spec | None | FLS — formally defines all behaviour |
| CVE handling | New compiler release | Backport to frozen LTS |
| Legal status | No certificate | TÜV SÜD ASIL-D + IEC 61508 SIL 4 |
| Coverage tools | System llvm-cov | Same SHA-pinned tarball as compiler |
| rust-toolchain.toml | `channel = "1.92.0"` | `channel = "ferrocene"` |
| Migration cost | — | 1 line change + license purchase |

---

## 11. Quick Reference

```bash
# Verify setup
bash scripts/verify_toolchain.sh

# Build
cargo build                          # debug
cargo build --release                # release (with safety profiles)

# Demo
cargo run                            # shows 10 safety scenarios

# Test
cargo test                           # run all tests
cargo test -- --nocapture            # with output
cargo test test_brake_command_mcdc   # run specific MC/DC test

# Coverage
bash scripts/run_coverage.sh         # full pipeline (66% gate)
bash scripts/run_coverage.sh --min-coverage 80   # stricter gate

# Sanitizers
bash scripts/run_sanitizers.sh       # ASan + UBSan evidence

# Documentation
cargo doc --open                     # build + open in browser

# The Ferrocene switch (production):
# Edit rust-toolchain.toml:
#   channel = "1.92.0"  →  channel = "ferrocene"
```

---

## References

| Document | Location |
|----------|----------|
| **Detailed Theory (Simple)** | [docs/ferrocene_theory.md](docs/ferrocene_theory.md) |
| **Manual Steps Theory** | [docs/manual_steps_walkthrough.md](docs/manual_steps_walkthrough.md) |
| Ferrocene Language Spec | https://ferrocene.dev/specification |
| Ferrous Systems (purchase) | https://ferrous-systems.com/ferrocene/ |
| S-CORE toolchain builder | https://github.com/eclipse-score/ferrocene_toolchain_builder |
| S-CORE workspace | `/home/lg/Desktop/score/ferrocene_guide/` |
| ISO 26262 (standard) | Via national standards body |
| Full Ferrocene document | See original document provided with this POC |

---

*POC created: 2026-05-14 | Compiler: rustc 1.92.0 | Production target: Ferrocene channel*
