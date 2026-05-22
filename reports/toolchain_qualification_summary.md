# Toolchain Qualification Summary
## ISO 26262 Part 8 — Tool Qualification Record

> **MANUAL STEP 51**: This document is the formal ISO 26262 Part 8 tool qualification record.  
> Every time the toolchain changes, this document must be updated and re-reviewed.  
> TÜV SÜD auditors check this document to verify the tool meets ASIL-D requirements.

---

## 1. Tool Identification

| Field                  | Value                                        |
|------------------------|----------------------------------------------|
| Tool Name              | Ferrocene (Rust Compiler)                    |
| Tool Version           | 779fbed05ae9e9fe2a04137929d99cc9b3d516fd     |
| Tool Provider          | Ferrous Systems GmbH + AdaCore              |
| Certification Body     | TÜV SÜD                                      |
| Standard               | ISO 26262:2018 (ASIL-D) + IEC 61508 (SIL 4) |
| Channel (POC)          | 1.92.0 (standard rustc — not certified)      |
| Channel (Production)   | ferrocene (change rust-toolchain.toml)       |

> **MANUAL STEP 52 (POC vs Production)**:  
> This POC uses channel `1.92.0`. The table above lists the production Ferrocene SHA.  
> For a real TÜV submission, replace all "1.92.0" references with the exact Ferrocene SHA.

---

## 2. Tool Classification (ISO 26262 Part 8, Clause 11)

| Classification Question                                  | Answer      |
|----------------------------------------------------------|-------------|
| Does incorrect tool output affect functional safety?     | YES         |
| Can incorrect output be prevented by other means?        | NO (compiler is the only translator) |
| Tool Confidence Level (TCL)                              | TCL-3 (highest) |
| Qualification method required                           | Tool Certification — independent body |
| Qualification method used                               | Ferrocene TÜV SÜD certificate        |

**Conclusion**: The Ferrocene compiler is a TCL-3 tool. Self-qualification would require running the full Ferrocene test suite (~50,000 test cases) ourselves. Using the TÜV SÜD-certified Ferrocene binary means this work is already done.

---

## 3. Tool Qualification Evidence

| Evidence Item                    | Location                                   | Status    |
|----------------------------------|--------------------------------------------|-----------|
| TÜV SÜD Certificate             | Ferrocene Qualification Kit (purchased)    | ⚠️ Pending license |
| Ferrocene Language Spec (FLS)    | ferrocene.dev/specification                | ✅ Public  |
| Ferrocene Safety Manual          | Ferrocene Qualification Kit                | ⚠️ Pending license |
| SHA-256 hash of compiler binary  | rust-toolchain.toml / MODULE.bazel         | ✅ Present |
| Compiler test suite results      | Ferrocene Qualification Kit                | ⚠️ Pending license |
| LTS CVE backporting procedure    | Ferrous Systems security policy            | ✅ Documented |

---

## 4. Platform Qualification Matrix

> **MANUAL STEP 53 (Platform-specific qualification)**:  
> Ferrocene is SEPARATELY qualified for each target platform. Using the wrong  
> target triple invalidates the ASIL-D argument even if the binary works correctly.

| Platform Target                        | OS         | Use Case                | Qualified |
|----------------------------------------|------------|-------------------------|-----------|
| `x86_64-unknown-linux-gnu`             | Linux x86  | CI + developer machines | ✅ Yes    |
| `aarch64-unknown-linux-gnu`            | Linux ARM  | ARM ECUs (AutoSD)       | ✅ Yes    |
| `x86_64-pc-nto-qnx800`                | QNX x86    | QNX QEMU                | ✅ Yes    |
| `aarch64-unknown-nto-qnx800`          | QNX ARM    | Production ECUs         | ✅ Yes    |

---

## 5. Known Gaps and Actions

| Gap                               | Impact                        | Action Required                         |
|-----------------------------------|-------------------------------|------------------------------------------|
| Ferrocene license not purchased   | Cannot use certified binary   | Purchase from Ferrous Systems            |
| TÜV certificate not in hand       | No legal proof of certification | Included with Ferrocene license         |
| Formal HARA not yet written       | Safety case incomplete        | Assign Safety Manager, write HARA       |

---

*Document last updated: 2026-05-14*  
*Next review: Before TÜV SÜD engagement*
