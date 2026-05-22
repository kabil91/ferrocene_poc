// =============================================================================
// FILE: src/safety/airbag_sensor.rs
// PURPOSE: Airbag deployment sensor fusion — ASIL-D component.
//
// This module demonstrates a SECOND safety-critical component: the airbag
// sensor that decides whether to fire a pyrotechnic squib.
//
// MANUAL STEP 11 (Sensor Fusion Pattern):
//   An airbag system reads from MULTIPLE independent sensors (redundancy).
//   ISO 26262 calls this "sensor diversity" — if sensor A fails high,
//   sensor B's vote prevents a spurious deployment.
//   The ResultFusion enum below models the 2-of-3 voting logic.
//
// MANUAL STEP 12 (Why This Cannot Use a Mutex Naively):
//   AirbagController is NOT Send (no `unsafe impl Send`).
//   This means Rust refuses to compile any code that moves it to another thread.
//   In an ECU with a safety task running on CPU0 and a comfort task on CPU1,
//   the airbag controller is PHYSICALLY restricted to CPU0 by the type system.
//   Ferrocene formally certifies that this proof is sound.
//
// ISO 26262 TRACEABILITY:
//   REQ-AIRBAG-001 → AirbagController::evaluate_deployment()
//   REQ-AIRBAG-002 → SensorVote::majority_vote()
//   REQ-AIRBAG-003 → DeploymentDecision enum
// =============================================================================

use std::marker::PhantomData;

// -----------------------------------------------------------------------
// SENSOR READING TYPES
// Each physical quantity has its own type — prevents unit confusion.
// -----------------------------------------------------------------------

/// Acceleration in G-force (1G = 9.81 m/s²)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct AccelerationG(pub f32);

/// Time since impact onset in milliseconds
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct TimeMs(pub u32);

// -----------------------------------------------------------------------
// SENSOR VOTE — 2-OF-3 MAJORITY VOTING
//
// MANUAL STEP 13 (Understanding Sensor Voting):
//   Three independent accelerometers read the crash event.
//   We only deploy if at least 2 of 3 agree the crash is severe enough.
//   This prevents a single faulty sensor from deploying the airbag into
//   a pedestrian during a parking maneuver.
//
//   The voting logic below is what MC/DC tests cover:
//     Condition A: sensor_1 votes deploy
//     Condition B: sensor_2 votes deploy
//     Condition C: sensor_3 votes deploy
//   Deploy if: (A && B) || (A && C) || (B && C)  [2-of-3 majority]
// -----------------------------------------------------------------------

/// A single sensor's binary deployment vote
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorVote {
    /// Sensor detected impact exceeding threshold → recommend deployment
    Deploy,
    /// Sensor reading below threshold → hold
    Hold,
}

impl SensorVote {
    /// Computes 2-of-3 majority vote from three independent sensors.
    ///
    /// # MC/DC Coverage
    /// The three-way OR condition requires MC/DC tests proving each pair
    /// independently controls the output. See integration_tests.rs for
    /// the complete 7-test MC/DC set for 2-of-3 voting.
    ///
    /// # Returns
    /// `Deploy` if 2 or more sensors vote to deploy, `Hold` otherwise.
    pub fn majority_vote(s1: SensorVote, s2: SensorVote, s3: SensorVote) -> SensorVote {
        // Convert votes to integers for concise counting
        // Each Deploy = 1, Hold = 0; threshold is 2
        let deploy_count = [s1, s2, s3]
            .iter()
            .filter(|&&v| v == SensorVote::Deploy)
            .count();

        // DECISION: 2-of-3 threshold
        // This is the condition covered by MC/DC tests.
        if deploy_count >= 2 {
            SensorVote::Deploy
        } else {
            SensorVote::Hold
        }
    }
}

// -----------------------------------------------------------------------
// DEPLOYMENT DECISION TREE
// -----------------------------------------------------------------------

/// The final deployment decision with full audit trail.
///
/// # Why Include Audit Data?
/// ISO 26262 Part 8 requires that safety decisions are traceable.
/// Storing the sensor readings alongside the decision means crash
/// data recorders can reproduce exactly why the airbag was (or wasn't) fired.
#[derive(Debug, PartialEq)]
pub enum DeploymentDecision {
    /// Airbag should be fired — squib activation command
    FireAirbag {
        /// Peak deceleration measured at impact (G-force)
        peak_acceleration: AccelerationG,
        /// Time from impact onset to fire command (milliseconds)
        time_to_decision: TimeMs,
    },
    /// Conditions not met — no action
    Hold,
    /// Sensor inconsistency detected — enter diagnostic mode
    SensorFault {
        /// Human-readable description of the fault condition
        reason: &'static str,
    },
}

// -----------------------------------------------------------------------
// AIRBAG CONTROLLER
// -----------------------------------------------------------------------

/// Evaluates sensor data and decides whether to fire the airbag squib.
///
/// # ASIL-D Safety Parameters
/// - `deploy_threshold_g`: Minimum deceleration to trigger deployment (e.g. 25G)
/// - `max_decision_time_ms`: Maximum time from impact onset to fire command (e.g. 30ms)
///
/// # Real-World Values (from typical automotive tuning):
/// - Frontal crash: ~30–50G peak, decision in 10–25ms
/// - Minor rear impact: 5–10G, should NOT deploy
pub struct AirbagController {
    deploy_threshold_g:   f32,
    max_decision_time_ms: u32,
    deployment_armed:     bool,
    _thread_lock:         PhantomData<*const ()>, // Strips Send and Sync automatically!
}

impl AirbagController {
    /// Creates a new AirbagController with the given deployment thresholds.
    ///
    /// # Safety
    /// Threshold values come from FMEA analysis and crash test calibration.
    /// Changing these values requires full re-verification of REQ-AIRBAG-001.
    pub fn new(deploy_threshold_g: f32, max_decision_time_ms: u32) -> Self {
        AirbagController {
            deploy_threshold_g,
            max_decision_time_ms,
            deployment_armed: true,  // Armed by default at ECU power-on
            _thread_lock:     PhantomData,
        }
    }

    /// Arms or disarms the deployment (used during manufacturing/service mode).
    pub fn set_armed(&mut self, armed: bool) {
        self.deployment_armed = armed;
    }

    // -----------------------------------------------------------------------
    // MAIN EVALUATION FUNCTION
    //
    // MANUAL STEP 14 (Multi-condition safety decision):
    //   This function combines:
    //     1. Sensor majority voting (2-of-3)
    //     2. Acceleration threshold comparison
    //     3. Timing constraint verification
    //     4. Armed state check
    //
    //   Each condition must be independently verified by MC/DC tests.
    //   See tests/integration_tests.rs: test_airbag_mcdc_set
    // -----------------------------------------------------------------------

    /// Evaluates three sensor readings and produces a deployment decision.
    ///
    /// # MC/DC Decision Logic
    /// Deploy if ALL of:
    ///   - Majority vote says deploy (2-of-3 sensors agree)    — Condition A
    ///   - Peak acceleration exceeds threshold                  — Condition B
    ///   - Decision made within timing budget                   — Condition C
    ///   - System is armed                                      — Condition D
    ///
    /// # Returns
    /// - `FireAirbag` if all conditions met
    /// - `Hold` if conditions not met
    /// - `SensorFault` if inputs appear corrupted
    pub fn evaluate_deployment(
        &self,
        sensor_1:          SensorVote,
        sensor_2:          SensorVote,
        sensor_3:          SensorVote,
        peak_acceleration: AccelerationG,
        time_to_decision:  TimeMs,
    ) -> DeploymentDecision {
        // -----------------------------------------------------------------------
        // GUARD: Disarmed state check
        // In service mode or manufacturing test, the system is disarmed.
        // No crash severity can override a disarmed state.
        // -----------------------------------------------------------------------
        if !self.deployment_armed {
            return DeploymentDecision::Hold;
        }

        // -----------------------------------------------------------------------
        // CONDITION A: Sensor majority vote
        // -----------------------------------------------------------------------
        let consensus = SensorVote::majority_vote(sensor_1, sensor_2, sensor_3);

        // -----------------------------------------------------------------------
        // SANITY CHECK: Detect impossible sensor combinations
        // All Deploy + acceleration below threshold → potential sensor fault
        // -----------------------------------------------------------------------
        if consensus == SensorVote::Deploy
            && peak_acceleration.0 < self.deploy_threshold_g * 0.1
        {
            // Three sensors agree on deploy but acceleration is negligible.
            // This is physically impossible — sensors are likely shorted together.
            return DeploymentDecision::SensorFault {
                reason: "All sensors vote deploy but acceleration is near-zero. \
                         Possible sensor bus fault or short circuit.",
            };
        }

        // -----------------------------------------------------------------------
        // MAIN DECISION: All four conditions evaluated
        // Condition A: majority vote → Deploy
        // Condition B: acceleration above deployment threshold
        // Condition C: decision made within timing budget
        // (Condition D — armed — already checked above)
        // -----------------------------------------------------------------------
        let should_fire =
            consensus == SensorVote::Deploy                  // Condition A
            && peak_acceleration.0 >= self.deploy_threshold_g // Condition B
            && time_to_decision.0 <= self.max_decision_time_ms; // Condition C

        if should_fire {
            DeploymentDecision::FireAirbag {
                peak_acceleration,
                time_to_decision,
            }
        } else {
            DeploymentDecision::Hold
        }
    }
}
