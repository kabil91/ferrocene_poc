// =============================================================================
// FILE: src/safety/mod.rs
// PURPOSE: Safety module entry point — re-exports all safety-critical submodules.
//
// MANUAL STEP 15 (Module Visibility Rules):
//   Rust's module system is the FIRST layer of access control.
//   Every type marked `pub` here is visible outside the `safety` module.
//   Types NOT marked `pub` are compiler-enforced private — not just a convention.
//
//   Ferrocene formally certifies this visibility model via the FLS (Ferrocene
//   Language Specification). An auditor can point to the FLS chapter on
//   visibility and prove that a private field cannot be accessed externally.
//
// MANUAL STEP 16 (Module structure rationale):
//   brake_controller.rs → ASIL-D brake actuation logic
//   airbag_sensor.rs    → ASIL-D airbag deployment decision
//   Each module is independently testable (unit tests within the file)
//   AND integration-tested (tests/integration_tests.rs).
//   ISO 26262 calls this "software unit testing" + "integration testing".
// =============================================================================

/// Brake-by-wire controller — ASIL-D
pub mod brake_controller;

/// Airbag deployment sensor fusion — ASIL-D
pub mod airbag_sensor;
