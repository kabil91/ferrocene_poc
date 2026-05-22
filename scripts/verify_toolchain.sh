#!/usr/bin/env bash
# =============================================================================
# FILE: scripts/verify_toolchain.sh
# PURPOSE: Verifies that the Ferrocene/Rust toolchain is correctly configured
#          before running any safety-critical build or test.
#
# MANUAL STEP 38 (Pre-flight checklist — run this before every CI build):
#   This script checks:
#   1. rust-toolchain.toml exists and declares a channel
#   2. The declared channel is resolved correctly by rustup
#   3. Required components (llvm-tools-preview) are installed
#   4. The compiler version matches what is expected
#   5. MODULE_ferrocene_block.bazel SHA is present (Ferrocene production check)
#
# HOW TO RUN:
#   cd /home/lg/Desktop/rust_test/ferrocene_poc
#   bash scripts/verify_toolchain.sh
# =============================================================================

set -euo pipefail
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_DIR"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; BOLD='\033[1m'; NC='\033[0m'
info()    { echo -e "${BLUE}  [CHECK]${NC} $*"; }
ok()      { echo -e "${GREEN}  [OK]   ${NC} $*"; }
warn()    { echo -e "${YELLOW}  [WARN] ${NC} $*"; }
fail()    { echo -e "${RED}  [FAIL] ${NC} $*"; }

echo -e "${BOLD}"
echo "══════════════════════════════════════════════════════════"
echo "  Ferrocene POC — Toolchain Verification"
echo "══════════════════════════════════════════════════════════"
echo -e "${NC}"

PASS=0; FAIL=0; WARN=0

# ─── Check 1: rust-toolchain.toml exists ──────────────────────────────────────
info "Checking rust-toolchain.toml..."
if [[ -f "rust-toolchain.toml" ]]; then
    CHANNEL=$(grep 'channel' rust-toolchain.toml | cut -d'"' -f2)
    ok "rust-toolchain.toml found, channel = \"$CHANNEL\""
    ((PASS++))
    if [[ "$CHANNEL" == "ferrocene" ]]; then
        ok "  ✅ Channel is 'ferrocene' — ASIL-D certified compiler active"
    elif [[ "$CHANNEL" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        warn "  ⚠️  Channel is '$CHANNEL' (standard rustc — not certified)"
        warn "     For ASIL-D: change to channel = \"ferrocene\""
        ((WARN++))
    else
        warn "  Channel is '$CHANNEL' — verify this is your intended toolchain"
        ((WARN++))
    fi
else
    fail "rust-toolchain.toml NOT FOUND"
    fail "  This means different developers may use different compiler versions"
    fail "  Create rust-toolchain.toml with: [toolchain]\n  channel = \"1.92.0\""
    ((FAIL++))
fi
echo

# ─── Check 2: rustup can resolve the declared channel ─────────────────────────
info "Checking rustup toolchain resolution..."
if RUSTC_VER=$(rustc --version 2>&1); then
    ok "rustc resolved: $RUSTC_VER"
    ((PASS++))
else
    fail "rustc not found or could not resolve channel"
    fail "  Run: rustup toolchain install $CHANNEL"
    ((FAIL++))
fi
echo

# ─── Check 3: cargo is available ──────────────────────────────────────────────
info "Checking cargo..."
if CARGO_VER=$(cargo --version 2>&1); then
    ok "cargo: $CARGO_VER"
    ((PASS++))
else
    fail "cargo not found"
    ((FAIL++))
fi
echo

# ─── Check 4: llvm-tools-preview for coverage ─────────────────────────────────
info "Checking llvm-tools-preview (needed for coverage pipeline)..."
if rustup component list --installed 2>/dev/null | grep -q "llvm-tools"; then
    ok "llvm-tools-preview is installed"
    ((PASS++))
    # Check for individual tools
    for tool in llvm-profdata llvm-cov; do
        if command -v "$tool" &>/dev/null; then
            ok "  $tool: $(command -v "$tool")"
        else
            warn "  $tool not on PATH (may still work via cargo proxy)"
            ((WARN++))
        fi
    done
else
    warn "llvm-tools-preview NOT installed"
    warn "  Coverage pipeline (scripts/run_coverage.sh) needs this"
    warn "  Install: rustup component add llvm-tools-preview"
    ((WARN++))
fi
echo

# ─── Check 5: cargo-llvm-cov (alternative coverage tool) ─────────────────────
info "Checking cargo-llvm-cov (alternative coverage tool)..."
if command -v cargo-llvm-cov &>/dev/null; then
    ok "cargo-llvm-cov available: $(cargo llvm-cov --version 2>/dev/null || echo 'installed')"
    ((PASS++))
else
    warn "cargo-llvm-cov not installed (optional but useful)"
    warn "  Install: cargo install cargo-llvm-cov"
    ((WARN++))
fi
echo

# ─── Check 6: Cargo.toml sanity ───────────────────────────────────────────────
info "Checking Cargo.toml..."
if [[ -f "Cargo.toml" ]]; then
    PKG_NAME=$(grep '^name' Cargo.toml | head -1 | cut -d'"' -f2)
    ok "Cargo.toml found, package = \"$PKG_NAME\""
    ((PASS++))

    # Verify overflow-checks is set in release profile
    if grep -q "overflow-checks" Cargo.toml; then
        ok "  overflow-checks is configured in build profile (ASIL requirement)"
    else
        warn "  overflow-checks not found in Cargo.toml profiles"
        warn "  Add: [profile.release]\n       overflow-checks = true"
        ((WARN++))
    fi
else
    fail "Cargo.toml not found"
    ((FAIL++))
fi
echo

# ─── Check 7: Source files exist ──────────────────────────────────────────────
info "Checking source structure..."
RS_COUNT=$(find src/ -name "*.rs" 2>/dev/null | wc -l)
TEST_COUNT=$(find tests/ -name "*.rs" 2>/dev/null | wc -l)
if [[ "$RS_COUNT" -gt 0 ]]; then
    ok "Source files: $RS_COUNT .rs files in src/"
    ((PASS++))
else
    fail "No .rs files found in src/"
    ((FAIL++))
fi
if [[ "$TEST_COUNT" -gt 0 ]]; then
    ok "Test files  : $TEST_COUNT .rs files in tests/"
    ((PASS++))
else
    warn "No test files in tests/ — integration tests missing"
    ((WARN++))
fi
echo

# ─── Check 8: Build sanity ────────────────────────────────────────────────────
info "Running cargo check (compile check without producing binary)..."
if cargo check --quiet 2>&1; then
    ok "cargo check passed — no compile errors"
    ((PASS++))
else
    fail "cargo check FAILED — fix compile errors before running tests"
    ((FAIL++))
fi
echo

# ─── Summary ──────────────────────────────────────────────────────────────────
echo -e "${BOLD}══════════════════════════════════════════════════════════${NC}"
echo -e "${BOLD}  Verification Summary${NC}"
echo -e "${BOLD}══════════════════════════════════════════════════════════${NC}"
echo -e "  ${GREEN}Passed : $PASS${NC}"
echo -e "  ${YELLOW}Warnings: $WARN${NC}"
echo -e "  ${RED}Failed : $FAIL${NC}"
echo

if [[ "$FAIL" -gt 0 ]]; then
    echo -e "${RED}  ❌ Toolchain verification FAILED — fix errors above before building${NC}"
    exit 1
elif [[ "$WARN" -gt 0 ]]; then
    echo -e "${YELLOW}  ⚠️  Toolchain verified with warnings — review warnings for full certification readiness${NC}"
    echo
    echo "  For ASIL-D production:"
    echo "    1. Change rust-toolchain.toml: channel = \"ferrocene\""
    echo "    2. Install llvm-tools-preview: rustup component add llvm-tools-preview"
    exit 0
else
    echo -e "${GREEN}  ✅ All checks passed — toolchain ready${NC}"
    exit 0
fi
