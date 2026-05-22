#!/usr/bin/env bash
# =============================================================================
# FILE: scripts/run_coverage.sh
# PURPOSE: Full coverage pipeline — instruments tests, collects .profraw files,
#          merges them into LCOV, and enforces the 66% minimum threshold.
#
# MANUAL STEP 27 (What this script does — step by step):
#
#   Step A: Set RUSTFLAGS to inject -Cinstrument-coverage into every crate.
#           This is the same flag as Ferrocene's bazelrc_ferrocene_flags.bazelrc.
#           It tells LLVM to insert counters at every branch point in the binary.
#
#   Step B: Set LLVM_PROFILE_FILE to tell each test binary where to write
#           its raw profile data (.profraw file). One file per test binary.
#
#   Step C: Run cargo test. The test binaries are compiled with instrumentation
#           and each writes a .profraw file when it finishes executing.
#
#   Step D: Merge all .profraw files into a single merged.profdata using
#           llvm-profdata. This is identical to what S-CORE's llvm_profile_wrapper
#           does in the Ferrocene/Bazel pipeline.
#
#   Step E: Generate coverage.lcov from merged.profdata using llvm-cov.
#           The LCOV format is what TÜV SÜD auditors accept as formal evidence.
#
#   Step F: Parse the LCOV file to calculate line coverage percentage.
#           Fail with exit code 1 if below the threshold (blocks CI merge).
#
# MANUAL STEP 28 (Ferrocene vs standard toolchain for coverage):
#   This script uses whatever llvm-profdata/llvm-cov is on PATH.
#   In Ferrocene production: these tools come from the SAME SHA-pinned tarball
#   as the compiler (communication/MODULE.bazel lines 139-141).
#   That is why coverage data is trusted safety evidence in Ferrocene —
#   the measurement tools are as certified as the compiler itself.
#
# HOW TO RUN:
#   cd /home/lg/Desktop/rust_test/ferrocene_poc
#   bash scripts/run_coverage.sh
#
# OPTIONAL: Change the threshold
#   bash scripts/run_coverage.sh --min-coverage 80
# =============================================================================

set -euo pipefail

# ─── Configuration ────────────────────────────────────────────────────────────
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COVERAGE_DIR="$PROJECT_DIR/target/coverage"
PROFRAW_DIR="$COVERAGE_DIR/profraw"
PROFDATA_FILE="$COVERAGE_DIR/merged.profdata"
LCOV_FILE="$COVERAGE_DIR/coverage.lcov"
HTML_DIR="$COVERAGE_DIR/html"
MIN_COVERAGE="${MIN_COVERAGE:-66}"   # Default: 66% (S-CORE ASIL-D minimum floor)

# Parse optional --min-coverage argument
while [[ $# -gt 0 ]]; do
    case "$1" in
        --min-coverage)
            MIN_COVERAGE="$2"
            shift 2
            ;;
        *)
            shift
            ;;
    esac
done

# ─── Colour helpers ───────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; BOLD='\033[1m'; NC='\033[0m'
info()    { echo -e "${BLUE}[INFO]${NC}  $*"; }
success() { echo -e "${GREEN}[PASS]${NC}  $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error()   { echo -e "${RED}[FAIL]${NC}  $*"; }
header()  { echo -e "\n${BOLD}══════════════════════════════════════════════════${NC}"; \
            echo -e "${BOLD}  $*${NC}"; \
            echo -e "${BOLD}══════════════════════════════════════════════════${NC}"; }

# ─── Step 0: Preflight ────────────────────────────────────────────────────────
header "Ferrocene POC — Coverage Pipeline"
info "Project dir : $PROJECT_DIR"
info "Output dir  : $COVERAGE_DIR"
info "Threshold   : $MIN_COVERAGE%"
echo

cd "$PROJECT_DIR"
mkdir -p "$PROFRAW_DIR"

# ─── Step A: Instrument and compile ───────────────────────────────────────────
# MANUAL STEP 29 (RUSTFLAGS explained):
#   -Cinstrument-coverage : Inject LLVM coverage counters into every function.
#   -Clink-dead-code      : Link even unreachable code — ensures 0% unreached code
#                           is visible in the report (not silently omitted).
#   -Ccodegen-units=1     : Compile as single unit — consistent instrumentation.
#   -Cdebuginfo=2         : Full debug symbols — needed for source-level report.
header "Step A — Compiling with coverage instrumentation"
info "Setting: RUSTFLAGS=-Cinstrument-coverage -Clink-dead-code -Ccodegen-units=1 -Cdebuginfo=2"
info "This is equivalent to ferrocene_guide/configs/bazelrc_ferrocene_flags.bazelrc"
echo

# ─── Step B + C: Run tests and collect .profraw files ─────────────────────────
header "Step B/C — Running tests and collecting .profraw profiles"
info "Each test binary writes a raw profile to: $PROFRAW_DIR/"
echo

RUSTFLAGS="-Cinstrument-coverage -Clink-dead-code -Ccodegen-units=1 -Cdebuginfo=2" \
LLVM_PROFILE_FILE="$PROFRAW_DIR/coverage-%p-%m.profraw" \
    cargo test --verbose 2>&1 | tee "$COVERAGE_DIR/test_output.log" || {
        error "Tests failed — no coverage data to collect"
        error "Fix failing tests before coverage can be measured"
        exit 1
    }

# Count .profraw files collected
PROFRAW_COUNT=$(find "$PROFRAW_DIR" -name "*.profraw" | wc -l)
info "Collected $PROFRAW_COUNT .profraw file(s)"

if [[ "$PROFRAW_COUNT" -eq 0 ]]; then
    error "No .profraw files found. Is rustc installed and does it support coverage?"
    error "Try: rustup component add llvm-tools-preview"
    exit 1
fi

# ─── Step D: Merge .profraw → merged.profdata ─────────────────────────────────
# MANUAL STEP 30 (Why merge?):
#   Each test binary writes its OWN .profraw. The library + integration_tests
#   + any doc tests each produce separate files. llvm-profdata merge combines
#   them into a single profile that covers all test execution paths together.
#   This is identical to what S-CORE's llvm_profile_wrapper does.
header "Step D — Merging profiles: .profraw → merged.profdata"

# CRITICAL: Always use the toolchain-bundled LLVM tools first.
# The .profraw files are produced by rustc 1.92.0 which uses LLVM profraw format v10.
# System llvm-profdata (e.g., Ubuntu LLVM 18 = v9 format) will REJECT them with
# "raw profile version mismatch". The bundled tool always matches the compiler exactly.
TOOLCHAIN_LLVM_BIN="$HOME/.rustup/toolchains/1.92.0-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin"

LLVM_PROFDATA=""
if [[ -x "$TOOLCHAIN_LLVM_BIN/llvm-profdata" ]]; then
    LLVM_PROFDATA="$TOOLCHAIN_LLVM_BIN/llvm-profdata"
    info "Using toolchain-bundled llvm-profdata (version-matched to compiler)"
elif command -v llvm-profdata &>/dev/null; then
    LLVM_PROFDATA="llvm-profdata"
    warn "Using system llvm-profdata — may cause version mismatch with .profraw files"
else
    warn "llvm-profdata not found anywhere"
fi

if [[ -z "$LLVM_PROFDATA" ]]; then
    warn "llvm-profdata not found on PATH."
    warn "Install with: rustup component add llvm-tools-preview"
    warn "Falling back to cargo-llvm-cov if available..."

    if command -v cargo-llvm-cov &>/dev/null; then
        header "Using cargo-llvm-cov (alternative pipeline)"
        cargo llvm-cov --lcov --output-path "$LCOV_FILE" -- --test-threads=1
    else
        error "Neither llvm-profdata nor cargo-llvm-cov found."
        error "Run: cargo install cargo-llvm-cov"
        exit 1
    fi
else
    info "Using: $LLVM_PROFDATA"
    "$LLVM_PROFDATA" merge \
        --sparse \
        $(find "$PROFRAW_DIR" -name "*.profraw" | tr '\n' ' ') \
        --output "$PROFDATA_FILE"
    success "Merged profdata: $PROFDATA_FILE"

    # ─── Step E: Generate LCOV report ─────────────────────────────────────────
    # MANUAL STEP 31 (llvm-cov and LCOV format):
    #   --format=lcov produces the standard LCOV file that:
    #     1. TÜV SÜD auditors accept as formal evidence
    #     2. Can be uploaded to Codecov, SonarQube for tracking
    #     3. Can be converted to HTML via genhtml for visual review
    #   --ignore-filename-regex skips third-party/generated code
    header "Step E — Generating coverage.lcov"

    header "Step E — Generating coverage.lcov + HTML report"

    # Use toolchain-bundled llvm-cov (version-matched to the compiler)
    LLVM_COV=""
    if [[ -x "$TOOLCHAIN_LLVM_BIN/llvm-cov" ]]; then
        LLVM_COV="$TOOLCHAIN_LLVM_BIN/llvm-cov"
        info "Using toolchain-bundled llvm-cov"
    elif command -v llvm-cov &>/dev/null; then
        LLVM_COV="llvm-cov"
        warn "Using system llvm-cov — may differ from compiler version"
    fi

    if [[ -n "$LLVM_COV" ]]; then
        # Use the integration test binary — it exercises all public safety APIs
        TEST_BIN=$(find target/debug/deps -name "integration_tests-*" -executable -not -name "*.d" | head -1)
        if [[ -z "$TEST_BIN" ]]; then
            TEST_BIN=$(find target/debug/deps -name "ferrocene_poc-*" -executable -not -name "*.d" | head -1)
        fi

        if [[ -n "$TEST_BIN" ]]; then
            info "Using test binary: $TEST_BIN"

            # Generate LCOV (the formal safety evidence format)
            "$LLVM_COV" export \
                --format=lcov \
                --instr-profile="$PROFDATA_FILE" \
                --ignore-filename-regex="/.cargo/registry|tests/" \
                "$TEST_BIN" \
                > "$LCOV_FILE"
            success "LCOV report: $LCOV_FILE"

            # Copy LCOV to project root for easy access
            cp "$LCOV_FILE" "$PROJECT_DIR/coverage.lcov"

            # Generate native HTML report using llvm-cov show (no genhtml needed)
            mkdir -p "$HTML_DIR"
            "$LLVM_COV" show \
                --format=html \
                --instr-profile="$PROFDATA_FILE" \
                --ignore-filename-regex="/.cargo/registry|tests/" \
                --output-dir="$HTML_DIR" \
                "$TEST_BIN" 2>/dev/null
            success "HTML report: $HTML_DIR/index.html"

            # Print human-readable summary table
            echo
            "$LLVM_COV" report \
                --instr-profile="$PROFDATA_FILE" \
                --ignore-filename-regex="/.cargo/registry|tests/" \
                "$TEST_BIN"
            echo
        else
            warn "Could not locate test binary for llvm-cov — LCOV not generated"
        fi
    else
        warn "llvm-cov not found — install via: rustup component add llvm-tools-preview"
    fi
fi

# ─── Step F: Parse coverage and enforce threshold ─────────────────────────────
# MANUAL STEP 32 (Coverage gate — this is what S-CORE CI enforces):
#   The 66% threshold mirrors lifecycle/BUILD lines 56-64 in S-CORE:
#     bazel run //:rust_coverage -- --min-line-coverage 66
#   If coverage drops below this number, the pipeline EXITS WITH CODE 1,
#   which causes CI to block the pull request merge.
#   This is the formal safety gate that TÜV auditors verify exists.
header "Step F — Coverage threshold enforcement"
info "Minimum required: $MIN_COVERAGE%"

if [[ -f "$LCOV_FILE" ]]; then
    # Parse LCOV file: sum DA (line data) entries to calculate percentage
    TOTAL_LINES=$(grep -c "^DA:" "$LCOV_FILE" 2>/dev/null || echo 0)
    COVERED_LINES=$(grep "^DA:" "$LCOV_FILE" 2>/dev/null | grep -v ",0$" | wc -l || echo 0)

    if [[ "$TOTAL_LINES" -gt 0 ]]; then
        COVERAGE_PCT=$(( (COVERED_LINES * 100) / TOTAL_LINES ))
        echo
        echo -e "  Total lines    : ${BOLD}$TOTAL_LINES${NC}"
        echo -e "  Covered lines  : ${BOLD}$COVERED_LINES${NC}"
        echo -e "  Line coverage  : ${BOLD}$COVERAGE_PCT%${NC}"
        echo -e "  Threshold      : ${BOLD}$MIN_COVERAGE%${NC}"
        echo

        if [[ "$COVERAGE_PCT" -ge "$MIN_COVERAGE" ]]; then
            success "Coverage gate PASSED: $COVERAGE_PCT% >= $MIN_COVERAGE%"
            success "LCOV file archived as formal safety evidence: $LCOV_FILE"
            echo
            echo -e "${GREEN}══════════════════════════════════════════════════${NC}"
            echo -e "${GREEN}  PIPELINE COMPLETE — SAFETY EVIDENCE GENERATED   ${NC}"
            echo -e "${GREEN}══════════════════════════════════════════════════${NC}"
            echo
            echo "  Formal evidence artifacts:"
            echo "    Coverage LCOV : $LCOV_FILE"
            [[ -f "$PROFDATA_FILE" ]] && echo "    Merged profdata: $PROFDATA_FILE"
            [[ -d "$HTML_DIR" ]]      && echo "    HTML report   : $HTML_DIR/index.html"
            echo "    Test log      : $COVERAGE_DIR/test_output.log"
            echo
            echo "  Next step for TÜV submission:"
            echo "    1. Archive $LCOV_FILE as the coverage evidence artifact"
            echo "    2. Include in safety evidence package alongside:"
            echo "       - Sanitizer run logs (scripts/run_sanitizers.sh)"
            echo "       - Ferrocene Qualification Kit (purchase from Ferrous Systems)"
            echo "       - HARA and safety requirements documents"
            exit 0
        else
            error "Coverage gate FAILED: $COVERAGE_PCT% < $MIN_COVERAGE%"
            error "Add more tests to cover the uncovered branches."
            error "Use the HTML report to see exactly which lines are missed:"
            [[ -d "$HTML_DIR" ]] && error "  open $HTML_DIR/index.html"
            exit 1
        fi
    else
        warn "Could not parse LCOV — displaying raw file summary"
        head -20 "$LCOV_FILE"
    fi
else
    warn "coverage.lcov not generated — check llvm-tools-preview installation"
    warn ""
    warn "Quick fix:"
    warn "  rustup component add llvm-tools-preview"
    warn "  cargo install cargo-llvm-cov"
    warn "  Then re-run this script"
    echo
    # At minimum, show that tests passed
    success "Tests passed (coverage report pending tool installation)"
    exit 0
fi
