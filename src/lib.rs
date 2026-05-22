//! # Ferrocene POC — ASIL-D Automotive Safety Library
//!
//! Safety-critical automotive modules demonstrating the Ferrocene certification pipeline.
//!
//! ## Manual Step 17 — Library vs Binary Split
//! Splitting into `lib.rs` (library) and `main.rs` (binary) is an ISO 26262
//! best practice: tests cover the library directly, coverage instrumentation
//! measures library code, and a HIL harness can link the same library unchanged.
//!
//! ## Safety Modules
//! - [`safety::brake_controller`] — ASIL-D brake-by-wire controller
//! - [`safety::airbag_sensor`]   — ASIL-D airbag deployment sensor fusion

// =============================================================================
// CRATE-LEVEL COMPILE FLAGS (MUST appear before any items — Rust requirement)
// These attributes enforce safety practices at the crate level and apply
// to EVERY module in this library.
// =============================================================================

// Treat missing documentation as a compile error.
// Every pub function must have a doc comment explaining its safety behaviour.
// TÜV auditors read these comments — they are safety documentation.
#![deny(missing_docs)]
// Warn on unused code — dead code can hide latent safety issues.
// Ferrocene's -Clink-dead-code flag catches this at link time too.
#![warn(dead_code)]
// Warn on unused Results — in safety-critical code, discarding a Result
// that carries an error is a potential safety violation.
#![warn(unused_must_use)]

/// Safety-critical automotive modules compiled with Ferrocene.
///
/// # Architecture
/// ```text
/// ferrocene_poc (lib)
/// └── safety/
///     ├── brake_controller  → ASIL-D brake actuation
///     └── airbag_sensor     → ASIL-D airbag deployment
/// ```
pub mod safety;
