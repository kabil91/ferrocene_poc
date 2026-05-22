// =============================================================================
// FILE: src/main.rs
// PURPOSE: Binary entry point — demonstration harness showing the safety
//          modules working together in a simulated ECU event loop.
//
// MANUAL STEP 18 (Running the POC):
//   Build and run:
//     cd /home/lg/Desktop/rust_test/ferrocene_poc
//     cargo run
//
//   Expected output shows:
//     - Normal braking scenario
//     - ABS engagement scenario
//     - Emergency stop scenario
//     - Sensor fault detection scenario
//     - Airbag deployment decision scenarios
//
// MANUAL STEP 19 (Observing compile-time safety):
//   Try to introduce a bug and observe the compiler catching it:
//     1. Try `let _x: SpeedKmh = BrakePressureKpa(50.0);`
//        → Type error: dimensional confusion caught at compile time
//     2. Try `let _ = controller.brake_command(speed, pressure);`
//        → Warning: must_use Result is being discarded
//     3. Try calling brake_command() from two threads simultaneously
//        → Compiler error: BrakeController is not Send
//
//   With Ferrocene, the compiler that catches these bugs is formally proven
//   to be correct. The FLS documents exactly why each rejection is sound.
// =============================================================================

// Allow unused imports in main for cleaner demonstration output
#![allow(unused_imports)]

use ferrocene_poc::safety::airbag_sensor::{
    AccelerationG, AirbagController, DeploymentDecision, SensorVote, TimeMs,
};
use ferrocene_poc::safety::brake_controller::{
    BrakeAction, BrakeController, BrakePressureKpa, SpeedKmh,
};

fn main() {
    // -----------------------------------------------------------------------
    // DEMONSTRATION 1: Brake Controller Scenarios
    // -----------------------------------------------------------------------
    println!("── BRAKE CONTROLLER SCENARIOS ──────────────────────────────────\n");

    // Create a calibrated brake controller
    // max_pressure = 400.0 kPa, min_pressure = 0.0 kPa
    // NOTE: min=0.0 means "pedal fully released" (0.0 kPa) is a valid reading.
    // A typical production ECU treats 0.0 kPa as "no pedal input", not a fault.
    let mut brake = BrakeController::new(400.0, 0.0);

    // Scenario 1: Normal driving — no pedal input
    let result = brake.brake_command(SpeedKmh(60.0), BrakePressureKpa(0.0));
    println!("Scenario 1 — Cruising at 60 km/h, no braking:");
    println!("  Speed: 60 km/h | Pressure: 0 kPa");
    println!("  Result: {:?}\n", result.unwrap());

    // Scenario 2: Light braking — speed high but pressure low (no ABS)
    let result = brake.brake_command(SpeedKmh(80.0), BrakePressureKpa(30.0));
    println!("Scenario 2 — Light braking at 80 km/h:");
    println!("  Speed: 80 km/h | Pressure: 30 kPa (below 50 kPa ABS threshold)");
    println!("  Result: {:?}\n", result.unwrap());

    // Scenario 3: ABS ENGAGEMENT — speed > 20 && pressure > 50
    let result = brake.brake_command(SpeedKmh(100.0), BrakePressureKpa(200.0));
    println!("Scenario 3 — Hard braking at 100 km/h:");
    println!("  Speed: 100 km/h | Pressure: 200 kPa (above 50 kPa ABS threshold)");
    println!("  Result: {:?}\n", result.unwrap());

    // Scenario 4: Sensor fault — pressure out of calibrated range
    let result = brake.brake_command(SpeedKmh(80.0), BrakePressureKpa(999.0));
    println!("Scenario 4 — Faulty pressure sensor (999 kPa > 400 kPa max):");
    println!("  Speed: 80 km/h | Pressure: 999 kPa (INVALID — above max)");
    match result {
        Ok(action) => println!("  Result: {:?} ← should not reach here", action),
        Err(e) => println!("  Error (expected): {:?}", e),
    }
    println!("  Controller state after fault: {:?}\n", brake.state());

    // Scenario 5: Post-fault command rejected
    let result = brake.brake_command(SpeedKmh(60.0), BrakePressureKpa(100.0));
    println!("Scenario 5 — Command after fault (controller locked):");
    match result {
        Ok(action) => println!("  Result: {:?} ← should not reach here", action),
        Err(e) => println!("  Error (expected — fault lock): {:?}", e),
    }
    println!();

    // Scenario 6: Emergency stop
    let mut brake2 = BrakeController::new(400.0, 0.0);
    let action = brake2.emergency_stop();
    println!("Scenario 6 — Emergency stop triggered:");
    println!("  Result: {:?}", action);
    println!("  Controller state: {:?}\n", brake2.state());

    // -----------------------------------------------------------------------
    // DEMONSTRATION 2: Airbag Deployment Decision
    // -----------------------------------------------------------------------
    println!("── AIRBAG CONTROLLER SCENARIOS ─────────────────────────────────\n");

    // Create airbag controller: deploy at 25G, within 30ms
    let airbag = AirbagController::new(25.0, 30);

    // Scenario 7: All sensors agree — severe crash → DEPLOY
    let decision = airbag.evaluate_deployment(
        SensorVote::Deploy,
        SensorVote::Deploy,
        SensorVote::Deploy,
        AccelerationG(45.0),
        TimeMs(15),
    );
    println!("Scenario 7 — Severe frontal crash (45G, all sensors agree):");
    println!("  Sensors: Deploy, Deploy, Deploy | Accel: 45G | Time: 15ms");
    println!("  Decision: {:?}\n", decision);

    // Scenario 8: 2-of-3 majority — one sensor failed high
    let decision = airbag.evaluate_deployment(
        SensorVote::Deploy,
        SensorVote::Deploy,
        SensorVote::Hold, // Sensor 3 didn't detect it (faulty or at bad angle)
        AccelerationG(32.0),
        TimeMs(20),
    );
    println!("Scenario 8 — Crash with one sensor disagreeing (2-of-3 rule):");
    println!("  Sensors: Deploy, Deploy, HOLD | Accel: 32G | Time: 20ms");
    println!("  Decision: {:?}\n", decision);

    // Scenario 9: Minor parking impact — should NOT deploy
    let decision = airbag.evaluate_deployment(
        SensorVote::Hold,
        SensorVote::Deploy, // One sensor overreacted
        SensorVote::Hold,
        AccelerationG(8.0),
        TimeMs(5),
    );
    println!("Scenario 9 — Minor parking impact (1-of-3 vote, low G):");
    println!("  Sensors: Hold, Deploy, Hold | Accel: 8G | Time: 5ms");
    println!("  Decision: {:?}\n", decision);

    // Scenario 10: Sensor fault (all vote deploy but acceleration near-zero)
    let decision = airbag.evaluate_deployment(
        SensorVote::Deploy,
        SensorVote::Deploy,
        SensorVote::Deploy,
        AccelerationG(0.2), // Near-zero — physically impossible combination
        TimeMs(5),
    );
    println!("Scenario 10 — Sensor fault detected (all deploy + near-zero G):");
    println!("  Sensors: Deploy, Deploy, Deploy | Accel: 0.2G | Time: 5ms");
    println!("  Decision: {:?}\n", decision);
}
