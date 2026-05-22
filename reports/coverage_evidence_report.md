# Coverage Evidence Report
## ASIL-D Coverage Measurement — Formal Safety Evidence

> **MANUAL STEP 54**: This document tracks coverage measurement results over time.  
> After every `bash scripts/run_coverage.sh` run, update the **Coverage Run Log** table below.  
> TÜV SÜD auditors will ask for a history of coverage results across all PRs.

---

## 1. Coverage Requirements

| Module              | ASIL Level | Required Coverage Type | Minimum % | Source          |
|---------------------|------------|------------------------|-----------|-----------------|
| `safety/brake_controller` | ASIL-D | MC/DC                 | 66% (line) | S-CORE safety plan |
| `safety/airbag_sensor`   | ASIL-D | MC/DC                 | 66% (line) | S-CORE safety plan |
| `main.rs` (binary)       | QM     | None mandatory         | —         | Not safety-critical |

> **MANUAL STEP 55 (Why 66%?)**:  
> ISO 26262 does not mandate a specific percentage — it mandates MC/DC for ASIL-D.  
> 66% is S-CORE's project-level minimum floor. It means: at minimum, 2/3 of every  
> executable line must be exercised by tests. TÜV may request a higher target  
> depending on the specific safety function being certified.

---

## 2. MC/DC Decision Coverage Map

The following functions contain boolean decisions requiring MC/DC test coverage.  
Each function below has its MC/DC test set in `tests/integration_tests.rs`.

| Function                                | Decision                              | Conditions | MC/DC Tests               |
|-----------------------------------------|---------------------------------------|------------|---------------------------|
| `BrakeController::pressure_is_valid()`  | `p >= min AND p <= max`               | 2          | T1, T2, T3 (lines 78-108) |
| `BrakeController::brake_command()`      | `speed > 20 AND pressure > 50`        | 2          | T1, T2, T3 (lines 126-160)|
| `SensorVote::majority_vote()`           | `count >= 2`                          | 3          | 8 tests (lines 211-240)   |
| `AirbagController::evaluate_deployment()` | `majority AND accel>=G AND time<=ms` | 3          | T1-T4 (lines 262-310)     |

---

## 3. Coverage Run Log

> **MANUAL STEP 56**: After each run of `bash scripts/run_coverage.sh`, copy the output  
> summary here. This creates the audit trail TÜV SÜD needs to verify consistent coverage.

| Date       | Commit SHA | Line % | Branch % | Gate Result | Run By        |
|------------|------------|--------|----------|-------------|---------------|
| 2026-05-14 | (initial)  | TBD    | TBD      | ⏳ Pending  | Run to populate |

**To populate this table:**
```bash
cd /home/lg/Desktop/rust_test/ferrocene_poc
bash scripts/run_coverage.sh
# Copy the output: "Line coverage: XX% (YY/ZZ lines)" into the table above
```

---

## 4. Coverage Tool Qualification

| Tool         | Version   | Source                              | Qualification Status           |
|--------------|-----------|-------------------------------------|--------------------------------|
| `llvm-cov`   | Matches rustc | Bundled with rust-toolchain.toml | Same toolchain chain (POC) |
| `llvm-profdata` | Matches rustc | Bundled with rust-toolchain.toml | Same toolchain chain (POC) |

> **Ferrocene Production Note** (MANUAL STEP 57):  
> In the S-CORE Ferrocene pipeline, `llvm-cov` and `llvm-profdata` come from the SAME  
> SHA-pinned tarball as the Ferrocene compiler (`communication/MODULE.bazel lines 139-141`).  
> This means the coverage measurement tools carry the **same TÜV SÜD qualification**  
> as the compiler itself. Coverage data is therefore trusted safety evidence.  
> In this POC, the tools come from the same `rust-toolchain.toml` pin — equivalent principle.

---

## 5. Known Gaps

| Gap                              | Impact                           | Resolution                           |
|----------------------------------|----------------------------------|--------------------------------------|
| MC/DC not yet measured separately | Cannot prove per-condition coverage | Enable `-Zcoverage-options=mcdc` on nightly |
| HTML report not auto-archived    | Auditors need browser-accessible view | Add `genhtml` to CI pipeline        |

---

*Document last updated: 2026-05-14*
