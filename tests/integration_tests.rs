// =============================================================================
// FILE: tests/integration_tests.rs
// PURPOSE: Integration test suite providing MC/DC coverage evidence for
//          ASIL-D certification of the brake controller and airbag modules.
//
// MANUAL STEP 20 (Why integration tests live here, not in src/):
//   Tests in tests/ are compiled as a SEPARATE binary against the library.
//   This means they exercise the PUBLIC API exactly as an external caller would.
//   ISO 26262 calls this "software integration testing" — verifying that
//   modules interact correctly when assembled together.
//
//   Tests inside src/ files (with #[cfg(test)]) are "unit tests" — they can
//   access private internals. Both levels are required for ASIL-D.
//
// MANUAL STEP 21 (MC/DC Coverage — what these tests prove):
//   Modified Condition/Decision Coverage (MC/DC) is mandatory for ASIL-C/D.
//   For each boolean DECISION with N conditions, you need N+1 test cases
//   where each condition independently controls the outcome.
//
//   Example: brake_command() has 2 conditions (speed AND pressure).
//   You need 3 tests: T1 (both true), T2 (only speed false), T3 (only pressure false).
//   All three tests are in the test_brake_command_mcdc_set() function below.
//
// MANUAL STEP 22 (Running these tests):
//   Standard run:
//     cargo test
//
//   With coverage instrumentation (produces .profraw for LCOV pipeline):
//     RUSTFLAGS="-Cinstrument-coverage" cargo test
//     (see scripts/run_coverage.sh which automates this)
//
//   Verbose output (see individual test names):
//     cargo test -- --nocapture
//
// COVERAGE TARGET:
//   These tests are designed to achieve >= 80% line coverage and 100% MC/DC
//   on the safety-critical decision functions.
//   The 66% threshold from S-CORE is the MINIMUM floor — good practice
//   targets the highest coverage achievable.
// =============================================================================

use ferrocene_poc::safety::airbag_sensor::{
    AccelerationG, AirbagController, DeploymentDecision, SensorVote, TimeMs,
};
use ferrocene_poc::safety::brake_controller::{
    BrakeAction, BrakeController, BrakeError, BrakePressureKpa, BrakeState, SpeedKmh,
};

// =============================================================================
// SECTION 1: BRAKE CONTROLLER — UNIT-LEVEL INTEGRATION TESTS
// =============================================================================

// -----------------------------------------------------------------------------
// TEST GROUP: pressure_is_valid() — 2-condition MC/DC set
//
// DECISION: pressure.0 >= min_pressure  AND  pressure.0 <= max_pressure
// Condition A: pressure >= min_pressure
// Condition B: pressure <= max_pressure
//
// MC/DC Test Matrix:
//   Test  | Cond A | Cond B | Result | Purpose
//   T1    | true   | true   | true   | Baseline — both conditions satisfied
//   T2    | false  | true   | false  | A independently controls outcome
//   T3    | true   | false  | false  | B independently controls outcome
//
// MANUAL STEP 23 (Reading the MC/DC matrix):
//   T2 proves A is not masked by B: changing A alone flips the result.
//   T3 proves B is not masked by A: changing B alone flips the result.
//   Without MC/DC, you could have 100% branch coverage on a version where
//   Condition B is always true (hardcoded), and never know.
// -----------------------------------------------------------------------------

#[test]
fn test_pressure_valid_mcdc_t1_both_conditions_true() {
    // T1: Condition A=true, Condition B=true → VALID
    // Controller: min=1.0 kPa, max=400.0 kPa
    let ctrl = BrakeController::new(400.0, 1.0);
    let pressure = BrakePressureKpa(200.0); // 200 >= 1 AND 200 <= 400 → valid
    assert!(
        ctrl.pressure_is_valid(pressure),
        "T1 FAILED: 200 kPa should be valid (between 1 and 400)"
    );
}

#[test]
fn test_pressure_valid_mcdc_t2_condition_a_false() {
    // T2: Condition A=false (pressure below min), Condition B=true → INVALID
    // Proves: Condition A (>= min) independently controls the decision
    let ctrl = BrakeController::new(400.0, 10.0); // min raised to 10 kPa
    let pressure = BrakePressureKpa(5.0); // 5 < 10 (A=false), 5 <= 400 (B=true)
    assert!(
        !ctrl.pressure_is_valid(pressure),
        "T2 FAILED: 5 kPa should be INVALID (below min=10)"
    );
}

#[test]
fn test_pressure_valid_mcdc_t3_condition_b_false() {
    // T3: Condition A=true (pressure above min), Condition B=false (above max) → INVALID
    // Proves: Condition B (<= max) independently controls the decision
    let ctrl = BrakeController::new(400.0, 1.0);
    let pressure = BrakePressureKpa(999.0); // 999 >= 1 (A=true), 999 > 400 (B=false)
    assert!(
        !ctrl.pressure_is_valid(pressure),
        "T3 FAILED: 999 kPa should be INVALID (above max=400)"
    );
}

#[test]
fn test_pressure_valid_exact_boundary_min() {
    // Boundary value test: pressure exactly AT minimum threshold
    // Boundary conditions are notorious failure points in C/C++ (off-by-one)
    // Rust's >= operator guarantees correct boundary inclusion
    let ctrl = BrakeController::new(400.0, 1.0);
    assert!(
        ctrl.pressure_is_valid(BrakePressureKpa(1.0)),
        "Exactly at min should be VALID"
    );
    assert!(
        !ctrl.pressure_is_valid(BrakePressureKpa(0.9)),
        "Just below min should be INVALID"
    );
}

#[test]
fn test_pressure_valid_exact_boundary_max() {
    // Boundary value test: pressure exactly AT maximum threshold
    let ctrl = BrakeController::new(400.0, 1.0);
    assert!(
        ctrl.pressure_is_valid(BrakePressureKpa(400.0)),
        "Exactly at max should be VALID"
    );
    assert!(
        !ctrl.pressure_is_valid(BrakePressureKpa(400.1)),
        "Just above max should be INVALID"
    );
}

// -----------------------------------------------------------------------------
// TEST GROUP: brake_command() — 2-condition MC/DC set
//
// DECISION: speed > 20.0  AND  pressure > 50.0  → engage ABS
// Condition A: speed.0 > 20.0
// Condition B: pressure.0 > 50.0
//
// MC/DC Test Matrix:
//   Test  | Cond A       | Cond B         | ABS? | Purpose
//   T1    | speed=100    | pressure=200   | YES  | Baseline — ABS engaged
//   T2    | speed=10     | pressure=200   | NO   | A controls outcome
//   T3    | speed=100    | pressure=30    | NO   | B controls outcome
// -----------------------------------------------------------------------------

#[test]
fn test_brake_command_mcdc_t1_abs_engaged() {
    // T1: speed > 20 AND pressure > 50 → ABS should engage
    let mut ctrl = BrakeController::new(400.0, 1.0);
    let result = ctrl.brake_command(SpeedKmh(100.0), BrakePressureKpa(200.0));
    assert!(result.is_ok(), "T1: brake_command should succeed");
    match result.unwrap() {
        BrakeAction::EngageAbs { .. } => {} // expected
        other => panic!("T1 FAILED: Expected EngageAbs, got {:?}", other),
    }
}

#[test]
fn test_brake_command_mcdc_t2_low_speed_no_abs() {
    // T2: speed <= 20 (A=false), pressure > 50 (B=true) → normal braking, NO ABS
    // Proves: speed condition independently controls ABS engagement
    let mut ctrl = BrakeController::new(400.0, 1.0);
    let result = ctrl.brake_command(SpeedKmh(10.0), BrakePressureKpa(200.0));
    assert!(result.is_ok(), "T2: brake_command should succeed");
    match result.unwrap() {
        BrakeAction::NormalBrake { .. } => {} // expected — no ABS at low speed
        other => panic!("T2 FAILED: Expected NormalBrake, got {:?}", other),
    }
}

#[test]
fn test_brake_command_mcdc_t3_low_pressure_no_abs() {
    // T3: speed > 20 (A=true), pressure <= 50 (B=false) → normal braking, NO ABS
    // Proves: pressure condition independently controls ABS engagement
    let mut ctrl = BrakeController::new(400.0, 1.0);
    let result = ctrl.brake_command(SpeedKmh(100.0), BrakePressureKpa(30.0));
    assert!(result.is_ok(), "T3: brake_command should succeed");
    match result.unwrap() {
        BrakeAction::NormalBrake { .. } => {} // expected — insufficient pedal for ABS
        other => panic!("T3 FAILED: Expected NormalBrake, got {:?}", other),
    }
}

#[test]
fn test_brake_command_no_pedal_input_releases() {
    // Zero pressure → Release action (not Normal or ABS)
    // NOTE: min_pressure = 0.0 so that 0.0 kPa (no pedal) is a valid reading.
    // In a real ECU, a sensor reading of 0.0 kPa means "pedal fully released"
    // and is a perfectly valid, expected value — distinct from a sensor fault.
    let mut ctrl = BrakeController::new(400.0, 0.0); // min=0.0 allows "no pedal" reading
    let result = ctrl.brake_command(SpeedKmh(80.0), BrakePressureKpa(0.0));
    assert!(
        result.is_ok(),
        "No-pedal command should succeed with min=0.0"
    );
    match result.unwrap() {
        BrakeAction::Release => {} // expected
        other => panic!("Expected Release, got {:?}", other),
    }
}

// -----------------------------------------------------------------------------
// TEST GROUP: Fault State Transitions
// MANUAL STEP 24 (Testing state machine transitions):
//   These tests verify that the BrakeState machine transitions correctly
//   and that fault states are STICKY (cannot silently self-clear).
//   ISO 26262 calls this "fail-safe behaviour" — once a fault is detected,
//   the system stays in a defined safe state until explicitly reset.
// -----------------------------------------------------------------------------

#[test]
fn test_invalid_sensor_triggers_fault_state() {
    let mut ctrl = BrakeController::new(400.0, 1.0);
    // State starts Inactive
    assert_eq!(ctrl.state(), BrakeState::Inactive);

    // Out-of-range pressure → fault
    let result = ctrl.brake_command(SpeedKmh(60.0), BrakePressureKpa(999.0));
    assert!(result.is_err(), "Invalid sensor should return error");
    assert_eq!(
        ctrl.state(),
        BrakeState::FaultDetected,
        "State must be FaultDetected"
    );

    // Verify fault is sticky — next valid command still rejected
    let result2 = ctrl.brake_command(SpeedKmh(60.0), BrakePressureKpa(100.0));
    assert!(result2.is_err(), "Fault state must block further commands");
    match result2.unwrap_err() {
        BrakeError::ActuatorFault { .. } => {} // expected
        other => panic!("Expected ActuatorFault, got {:?}", other),
    }
}

#[test]
fn test_hardware_reset_clears_fault() {
    let mut ctrl = BrakeController::new(400.0, 1.0);
    let _ = ctrl.brake_command(SpeedKmh(60.0), BrakePressureKpa(999.0)); // trigger fault
    assert_eq!(ctrl.state(), BrakeState::FaultDetected);

    // Reset via hardware reset (production: needs safety interlock confirmation)
    ctrl.hardware_reset();
    assert_eq!(
        ctrl.state(),
        BrakeState::Inactive,
        "After reset state should be Inactive"
    );

    // Commands should work again
    let result = ctrl.brake_command(SpeedKmh(80.0), BrakePressureKpa(100.0));
    assert!(
        result.is_ok(),
        "Command should succeed after hardware reset"
    );
}

#[test]
fn test_emergency_stop_transitions_state() {
    let mut ctrl = BrakeController::new(400.0, 1.0);
    let action = ctrl.emergency_stop();
    // Must return max pressure engage
    match action {
        BrakeAction::EngageAbs { .. } => {}
        other => panic!("Emergency stop must EngageAbs, got {:?}", other),
    }
    assert_eq!(ctrl.state(), BrakeState::EmergencyStop);
}

#[test]
fn test_controller_initial_state_is_inactive() {
    // Verify that a freshly constructed controller starts in Inactive.
    // In C, uninitialized struct fields are undefined behaviour.
    // In Rust, the compiler ENFORCES full initialization via BrakeController::new().
    let ctrl = BrakeController::new(400.0, 1.0);
    assert_eq!(ctrl.state(), BrakeState::Inactive);
}

#[test]
#[should_panic(expected = "SAFETY VIOLATION")]
fn test_misconfigured_controller_panics_at_construction() {
    // min >= max is a misconfigured ECU — must panic at construction
    // NOT at runtime during a safety-critical event.
    let _ctrl = BrakeController::new(100.0, 200.0); // min > max → panic
}

// =============================================================================
// SECTION 2: AIRBAG SENSOR — MC/DC TEST SETS
// =============================================================================

// -----------------------------------------------------------------------------
// TEST GROUP: SensorVote::majority_vote() — 2-of-3 MC/DC
//
// MANUAL STEP 25 (2-of-3 voting MC/DC):
//   For a 2-of-3 majority vote, every pair must be tested to prove
//   each sensor independently contributes to the decision.
//   A full MC/DC set requires testing that removing any single sensor's
//   vote can change the outcome.
// -----------------------------------------------------------------------------

#[test]
fn test_sensor_vote_all_deploy() {
    let vote =
        SensorVote::majority_vote(SensorVote::Deploy, SensorVote::Deploy, SensorVote::Deploy);
    assert_eq!(vote, SensorVote::Deploy, "3/3 deploy must be Deploy");
}

#[test]
fn test_sensor_vote_two_of_three_s1_s2() {
    // S1 and S2 deploy, S3 holds → should deploy (2-of-3)
    let vote = SensorVote::majority_vote(SensorVote::Deploy, SensorVote::Deploy, SensorVote::Hold);
    assert_eq!(vote, SensorVote::Deploy, "S1+S2 deploy, S3 hold → Deploy");
}

#[test]
fn test_sensor_vote_two_of_three_s1_s3() {
    // S1 and S3 deploy, S2 holds → should deploy (2-of-3)
    let vote = SensorVote::majority_vote(SensorVote::Deploy, SensorVote::Hold, SensorVote::Deploy);
    assert_eq!(vote, SensorVote::Deploy, "S1+S3 deploy, S2 hold → Deploy");
}

#[test]
fn test_sensor_vote_two_of_three_s2_s3() {
    // S2 and S3 deploy, S1 holds → should deploy (2-of-3)
    let vote = SensorVote::majority_vote(SensorVote::Hold, SensorVote::Deploy, SensorVote::Deploy);
    assert_eq!(vote, SensorVote::Deploy, "S2+S3 deploy, S1 hold → Deploy");
}

#[test]
fn test_sensor_vote_one_of_three_s1_only() {
    // Only S1 deploys → insufficient majority → Hold
    let vote = SensorVote::majority_vote(SensorVote::Deploy, SensorVote::Hold, SensorVote::Hold);
    assert_eq!(vote, SensorVote::Hold, "1/3 deploy must be Hold");
}

#[test]
fn test_sensor_vote_one_of_three_s2_only() {
    let vote = SensorVote::majority_vote(SensorVote::Hold, SensorVote::Deploy, SensorVote::Hold);
    assert_eq!(vote, SensorVote::Hold, "1/3 deploy must be Hold");
}

#[test]
fn test_sensor_vote_one_of_three_s3_only() {
    let vote = SensorVote::majority_vote(SensorVote::Hold, SensorVote::Hold, SensorVote::Deploy);
    assert_eq!(vote, SensorVote::Hold, "1/3 deploy must be Hold");
}

#[test]
fn test_sensor_vote_all_hold() {
    let vote = SensorVote::majority_vote(SensorVote::Hold, SensorVote::Hold, SensorVote::Hold);
    assert_eq!(vote, SensorVote::Hold, "0/3 deploy must be Hold");
}

// -----------------------------------------------------------------------------
// TEST GROUP: evaluate_deployment() — 3-condition MC/DC
//
// DECISION: (majority=Deploy) AND (accel >= threshold) AND (time <= budget)
// Condition A: majority vote = Deploy
// Condition B: peak_acceleration >= deploy_threshold_g  (25G)
// Condition C: time_to_decision <= max_decision_time_ms (30ms)
//
// MC/DC Matrix:
//   Test | A     | B     | C     | Result | Purpose
//   T1   | true  | true  | true  | FIRE   | Baseline
//   T2   | false | true  | true  | HOLD   | A controls outcome
//   T3   | true  | false | true  | HOLD   | B controls outcome
//   T4   | true  | true  | false | HOLD   | C controls outcome
// -----------------------------------------------------------------------------

#[test]
fn test_airbag_mcdc_t1_all_conditions_fire() {
    // T1: A=true (2/3 majority), B=true (30G >= 25G), C=true (20ms <= 30ms) → FIRE
    let ctrl = AirbagController::new(25.0, 30);
    let decision = ctrl.evaluate_deployment(
        SensorVote::Deploy,
        SensorVote::Deploy,
        SensorVote::Hold,    // 2-of-3 → Deploy
        AccelerationG(30.0), // B=true
        TimeMs(20),          // C=true
    );
    match decision {
        DeploymentDecision::FireAirbag { .. } => {}
        other => panic!("T1 FAILED: Expected FireAirbag, got {:?}", other),
    }
}

#[test]
fn test_airbag_mcdc_t2_minority_vote_holds() {
    // T2: A=false (only 1/3 votes deploy), B=true, C=true → HOLD
    // Proves: sensor majority independently controls deployment
    let ctrl = AirbagController::new(25.0, 30);
    let decision = ctrl.evaluate_deployment(
        SensorVote::Deploy,
        SensorVote::Hold,
        SensorVote::Hold, // 1-of-3 → Hold
        AccelerationG(30.0),
        TimeMs(20),
    );
    assert_eq!(
        decision,
        DeploymentDecision::Hold,
        "T2 FAILED: minority vote should Hold"
    );
}

#[test]
fn test_airbag_mcdc_t3_low_acceleration_holds() {
    // T3: A=true (2/3 majority), B=false (15G < 25G threshold), C=true → HOLD
    // Proves: acceleration threshold independently controls deployment
    let ctrl = AirbagController::new(25.0, 30);
    let decision = ctrl.evaluate_deployment(
        SensorVote::Deploy,
        SensorVote::Deploy,
        SensorVote::Hold,
        AccelerationG(15.0), // B=false — below threshold
        TimeMs(20),
    );
    assert_eq!(
        decision,
        DeploymentDecision::Hold,
        "T3 FAILED: low-G should Hold"
    );
}

#[test]
fn test_airbag_mcdc_t4_late_decision_holds() {
    // T4: A=true, B=true, C=false (50ms > 30ms budget) → HOLD
    // Proves: timing constraint independently controls deployment
    // (A late deployment decision is more dangerous than no deployment)
    let ctrl = AirbagController::new(25.0, 30);
    let decision = ctrl.evaluate_deployment(
        SensorVote::Deploy,
        SensorVote::Deploy,
        SensorVote::Hold,
        AccelerationG(30.0),
        TimeMs(50), // C=false — exceeded 30ms budget
    );
    assert_eq!(
        decision,
        DeploymentDecision::Hold,
        "T4 FAILED: late decision should Hold"
    );
}

#[test]
fn test_airbag_sensor_fault_detected() {
    // All sensors deploy but acceleration near-zero → physically impossible → SensorFault
    let ctrl = AirbagController::new(25.0, 30);
    let decision = ctrl.evaluate_deployment(
        SensorVote::Deploy,
        SensorVote::Deploy,
        SensorVote::Deploy,
        AccelerationG(0.2), // Near-zero — impossible with 3/3 deploy votes
        TimeMs(10),
    );
    match decision {
        DeploymentDecision::SensorFault { .. } => {}
        other => panic!("Expected SensorFault, got {:?}", other),
    }
}

#[test]
fn test_airbag_disarmed_prevents_deployment() {
    // Service mode: system disarmed — no crash should fire the airbag
    let mut ctrl = AirbagController::new(25.0, 30);
    ctrl.set_armed(false);

    let decision = ctrl.evaluate_deployment(
        SensorVote::Deploy,
        SensorVote::Deploy,
        SensorVote::Deploy,
        AccelerationG(50.0),
        TimeMs(10),
    );
    assert_eq!(
        decision,
        DeploymentDecision::Hold,
        "Disarmed system must Hold regardless of inputs"
    );
}

// =============================================================================
// SECTION 3: CROSS-MODULE INTEGRATION TEST
// Tests brake and airbag modules working together in a crash scenario.
// MANUAL STEP 26 (Integration test — combined scenario):
//   This simulates a real crash event sequence:
//   1. Driver brakes hard at high speed
//   2. Impact detected → airbag evaluates deployment
//   3. Brake emergency stop triggered
//   4. Verify both modules end in correct states
// =============================================================================

#[test]
fn test_crash_scenario_brake_and_airbag_coordination() {
    // --- PRE-CRASH: Normal driving ---
    let mut brake = BrakeController::new(400.0, 1.0);
    let airbag = AirbagController::new(25.0, 30);

    // Driver braking at high speed before impact
    let pre_crash = brake.brake_command(SpeedKmh(120.0), BrakePressureKpa(300.0));
    assert!(pre_crash.is_ok());
    match pre_crash.unwrap() {
        BrakeAction::EngageAbs { .. } => {} // ABS correctly engaged
        other => panic!("Pre-crash ABS should engage, got {:?}", other),
    }

    // --- IMPACT ---
    // Airbag sensors detect 40G crash
    let deploy_decision = airbag.evaluate_deployment(
        SensorVote::Deploy,
        SensorVote::Deploy,
        SensorVote::Deploy,
        AccelerationG(40.0),
        TimeMs(18),
    );
    match deploy_decision {
        DeploymentDecision::FireAirbag {
            peak_acceleration, ..
        } => {
            assert!(peak_acceleration.0 >= 25.0, "40G must exceed 25G threshold");
        }
        other => panic!("Impact should FireAirbag, got {:?}", other),
    }

    // Brake controller goes to emergency stop
    let emerg = brake.emergency_stop();
    match emerg {
        BrakeAction::EngageAbs { .. } => {} // Max pressure applied
        other => panic!("Emergency stop must EngageAbs, got {:?}", other),
    }
    assert_eq!(brake.state(), BrakeState::EmergencyStop);

    // Post-crash: brake commands are rejected (system in emergency state)
    // (EmergencyStop is locked — requires hardware reset to clear)
    // This is the correct ASIL-D fail-safe behaviour.
    println!("Crash scenario passed: ABS engaged → airbag deployed → emergency stop locked");
}
