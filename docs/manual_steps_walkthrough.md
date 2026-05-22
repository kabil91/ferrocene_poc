# Walkthrough: The Theory Behind the Manual Steps

This document explains the **theoretical safety requirements** behind the 60 Manual Steps mentioned in the source code comments.

---

## Group 1: Toolchain & Build (Steps 1–5)
**Location:** `rust-toolchain.toml`, `Cargo.toml`
*   **The Theory:** **Tool Reproducibility.** ISO 26262 requires that every binary produced for a car must be reproducible. 
*   **The Manual Step:** By pinning the version (Step 1) and setting `codegen-units = 1` (Step 5), we ensure that the compiler doesn't use "random" optimizations. The binary produced on your laptop will be bit-for-bit identical to the one in the car.

## Group 2: Memory Safety & Types (Steps 6–10)
**Location:** `src/safety/brake_controller.rs`
*   **The Theory:** **Prevention of Systematic Faults.** Most car crashes caused by software come from "Undefined Behavior" (like a pointer pointing to the wrong memory).
*   **The Manual Step:** We use "Newtypes" like `BrakePressureKpa(f32)` (Step 6). This makes it physically impossible for the compiler to mix up "Brake Pressure" with "Vehicle Speed." In C, they are both just numbers; in Rust/Ferrocene, they are different "Realities."

## Group 3: Sensor Fusion & Voting (Steps 11–14)
**Location:** `src/safety/airbag_sensor.rs`
*   **The Theory:** **Redundancy & Fault Tolerance.** Hardware fails. Sensors break.
*   **The Manual Step:** We implement 2-of-3 majority voting (Step 13). If one sensor says "Deploy" but the other two say "Hold," the system stays safe. Ferrocene ensures that this logic is compiled exactly as written without "clever" optimizations that might skip a vote.

## Group 4: Software Architecture (Steps 15–19)
**Location:** `src/lib.rs`, `src/main.rs`
*   **The Theory:** **Encapsulation & Interface Safety.** 
*   **The Manual Step:** We separate the "Safety Logic" (the library) from the "Demonstration" (the binary). Step 17 explains that by doing this, we can run "Unit Tests" on the logic without needing a real car or even a real dashboard.

## Group 5: MC/DC Testing (Steps 20–26)
**Location:** `tests/integration_tests.rs`
*   **The Theory:** **Structural Coverage.** How do you prove you tested everything?
*   **The Manual Step:** Step 21 explains MC/DC. We don't just test if the brakes work; we test the "Boundary Conditions." We test exactly `400.0` kPa and `400.1` kPa to prove the "Edge" of the safety logic is solid.

## Group 6: The Evidence Pipeline (Steps 27–37)
**Location:** `scripts/run_coverage.sh`, `scripts/run_sanitizers.sh`
*   **The Theory:** **Evidence of Absence (of bugs).** An auditor doesn't believe you; they believe the reports.
*   **The Manual Step:** Step 32 is the "Safety Gate." If the coverage is 65% (below the 66% limit), the pipeline kills itself. This is "Automated Enforcement" of safety policy.

## Group 7: Continuous Integration (Steps 39–50)
**Location:** `.github/workflows/ferrocene_pipeline.yml`
*   **The Theory:** **Independence of Verification.** 
*   **The Manual Step:** Step 39 answers the auditor's favorite question: "How do you know no one cheated?" Since the pipeline runs on every single change and generates the report automatically, the evidence is "Fresh" and "Untampered."

## Group 8: Qualification & Gaps (Steps 51–60)
**Location:** `reports/*.md`
*   **The Theory:** **Tool Qualification.** 
*   **The Manual Step:** Step 51 is your "Compliance Record." It lists the legal SHA-256 hashes of the compiler. If a hacker tried to swap your Ferrocene compiler for a "regular" one, the hash wouldn't match, and the audit would fail immediately.

---
*Manual Step Walkthrough v1.0*
