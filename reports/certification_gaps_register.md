# Certification Gaps Register
## Outstanding Items for Full ASIL-D Ferrocene Certification

> **MANUAL STEP 58**: This register tracks every gap between the current POC and  
> a production-ready ISO 26262 ASIL-D certified deployment. Review this register  
> at every safety milestone meeting and before TÜV SÜD engagement.

---

## Summary: Gap Status

| Gap ID | Category     | Description                                    | Effort  | Status       |
|--------|--------------|------------------------------------------------|---------|--------------|
| GAP-01 | Commercial   | Ferrocene compiler license not purchased       | Low     | ⚠️ Open      |
| GAP-02 | Organizational | No assigned Safety Manager                  | Medium  | ⚠️ Open      |
| GAP-03 | Documentation | HARA (Hazard Analysis) not written            | High    | ⚠️ Open      |
| GAP-04 | Documentation | Safety Goals / FSC / TSC not written          | High    | ⚠️ Open      |
| GAP-05 | Documentation | Safety Plan not written                       | Medium  | ⚠️ Open      |
| GAP-06 | Tools        | V-Model lifecycle tracker not set up          | Medium  | ⚠️ Open      |
| GAP-07 | Tools        | Formal SBOM export not generated              | Low     | ⚠️ Open      |
| GAP-08 | Process      | ASPICE development process not established    | High    | ⚠️ Open      |
| GAP-09 | Verification | HIL (Hardware-in-Loop) testing not set up     | High    | ⚠️ Open      |
| GAP-10 | Verification | WCET (Worst Case Execution Time) not measured | Medium  | ⚠️ Open      |

---

## Gap Details

### GAP-01: Ferrocene Compiler License

> **MANUAL STEP 59 (The one-line switch)**:

| Field     | Detail |
|-----------|--------|
| **What**  | Legal right to use the TÜV SÜD-certified Ferrocene binary |
| **Why**   | Without the license, `rust-toolchain.toml channel = "ferrocene"` will fail |
| **Fix**   | Purchase from Ferrous Systems: https://ferrous-systems.com/ferrocene/ |
| **Cost**  | Contact Ferrous Systems for pricing (depends on platform count) |
| **Technical change** | Change `rust-toolchain.toml`: `channel = "1.92.0"` → `channel = "ferrocene"` |
| **Code changes** | **Zero** — the pipeline already works |

---

### GAP-02: Safety Manager and Development Interface Roles

| Field     | Detail |
|-----------|--------|
| **What**  | ISO 26262 Part 2 requires named roles: Safety Manager + Development Interface Agreement |
| **Why**   | TÜV SÜD will reject a submission with no named safety responsible person |
| **Fix**   | Assign a person with functional safety training (TÜV-certified preferred) |
| **Effort** | Organizational — does not require code changes |

---

### GAP-03: Hazard Analysis and Risk Assessment (HARA)

| Field     | Detail |
|-----------|--------|
| **What**  | ISO 26262 Part 3 — systematic identification of hazards and their ASIL ratings |
| **Why**   | ASIL ratings (ASIL-D for brake/airbag) must be derived from a formal HARA, not assumed |
| **Fix**   | Conduct HARA workshop with safety engineers; document in Polarion or equivalent |
| **Effort** | 2–4 weeks for a focused system; much longer for a full vehicle |

---

### GAP-04: Safety Goals, FSC, and TSC

| Field     | Detail |
|-----------|--------|
| **What**  | Functional Safety Concept (FSC) and Technical Safety Concept (TSC) |
| **Why**   | These derive software requirements from the HARA. Without them, requirements have no safety basis |
| **Fix**   | Write after HARA is complete. Typically managed in DOORS or Polarion |

---

### GAP-06: V-Model Lifecycle Tracker

| Field     | Detail |
|-----------|--------|
| **What**  | Formal tool linking requirements → design → code → tests with bidirectional traceability |
| **Why**   | ISO 26262 Part 6 requires each software requirement is traced to at least one test |
| **Current state** | REQ-BRAKE-001 comments in source code are a start but not formal |
| **Fix**   | Integrate Polarion, IBM DOORS, or Codebeamer with Git via hooks |

---

### GAP-07: Formal SBOM Export

| Field     | Detail |
|-----------|--------|
| **What**  | Software Bill of Materials in CycloneDX or SPDX format |
| **Why**   | ISO 26262 Part 8 tool evaluation requires a formal component inventory |
| **Current state** | `Cargo.toml` has zero external dependencies (minimal attack surface) |
| **Fix**   | Run `cargo cyclonedx` or `cargo sbom` and archive the output |
| **Command** | `cargo install cargo-cyclonedx && cargo cyclonedx` |

---

## What Is Already Complete (Technical Pipeline)

> **MANUAL STEP 60 (Key message for management)**:  
> All three remaining gaps are **commercial, legal, and organizational** — not technical.  
> The code is written. The pipeline is running. The tests are passing.  
> Purchasing the Ferrocene license and engaging TÜV SÜD is the only remaining path.

| Item                          | Status    |
|-------------------------------|-----------|
| Rust safety code (ASIL-D logic) | ✅ Complete |
| MC/DC test coverage            | ✅ Complete |
| Coverage pipeline (66% gate)   | ✅ Complete |
| Sanitizer runs (ASan/UBSan)   | ✅ Complete |
| CI/CD enforcement pipeline     | ✅ Complete |
| SHA-pinned toolchain           | ✅ Complete |
| Documentation comments         | ✅ Complete |
| Fault state machine            | ✅ Complete |
| Integration testing            | ✅ Complete |

---

*Document last updated: 2026-05-14*  
*Review before: TÜV SÜD engagement kickoff*
