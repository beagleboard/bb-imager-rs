#!/bin/bash
# Platform-specific E2E test runner for BeagleBoard Imager
#
# This script runs E2E tests for all three platforms (Linux, Windows, macOS)
# and generates detailed test reports.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Detect platform
detect_platform() {
    case "$(uname -s)" in
        Linux*)     PLATFORM=linux;;
        Darwin*)    PLATFORM=macos;;
        MINGW*|MSYS*|CYGWIN*)    PLATFORM=windows;;
        *)          PLATFORM=unknown;;
    esac
    echo "${GREEN}Detected platform: ${PLATFORM}${NC}"
}

# Print usage
print_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Options:
    --sd            Run SD card flashing tests only
    --bcf           Run BCF (BeagleConnect Freedom) tests only
    --dfu           Run DFU tests only
    --all           Run all tests (default)
    --verbose       Show verbose output
    --report        Generate detailed test report
    --help          Show this help message

Examples:
    $0 --all                 # Run all E2E tests
    $0 --sd --verbose        # Run SD tests with verbose output
    $0 --bcf --dfu           # Run BCF and DFU tests
EOF
}

# Default options
RUN_SD=false
RUN_BCF=false
RUN_DFU=false
RUN_ALL=true
VERBOSE=false
GENERATE_REPORT=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --sd)
            RUN_SD=true
            RUN_ALL=false
            shift
            ;;
        --bcf)
            RUN_BCF=true
            RUN_ALL=false
            shift
            ;;
        --dfu)
            RUN_DFU=true
            RUN_ALL=false
            shift
            ;;
        --all)
            RUN_ALL=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --report)
            GENERATE_REPORT=true
            shift
            ;;
        --help)
            print_usage
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            print_usage
            exit 1
            ;;
    esac
done

# Detect platform
detect_platform

# If RUN_ALL is true, enable all test suites
if [ "$RUN_ALL" = true ]; then
    RUN_SD=true
    RUN_BCF=true
    RUN_DFU=true
fi

# Set verbose flag
CARGO_VERBOSE=""
if [ "$VERBOSE" = true ]; then
    CARGO_VERBOSE="--verbose"
fi

# Change to the e2e-tests directory
cd "$(dirname "$0")"

# Test report file
REPORT_FILE="test-report-${PLATFORM}-$(date +%Y%m%d-%H%M%S).txt"

# Function to run tests
run_tests() {
    local feature=$1
    local name=$2

    echo -e "\n${YELLOW}Running ${name} tests on ${PLATFORM}...${NC}"

    if [ "$GENERATE_REPORT" = true ]; then
        cargo test --test e2e --features "${feature}" ${CARGO_VERBOSE} 2>&1 | tee -a "$REPORT_FILE"
        TEST_RESULT=${PIPESTATUS[0]}
    else
        cargo test --test e2e --features "${feature}" ${CARGO_VERBOSE}
        TEST_RESULT=$?
    fi

    if [ $TEST_RESULT -eq 0 ]; then
        echo -e "${GREEN}✓ ${name} tests PASSED${NC}"
        return 0
    else
        echo -e "${RED}✗ ${name} tests FAILED${NC}"
        return 1
    fi
}

# Initialize report
if [ "$GENERATE_REPORT" = true ]; then
    cat > "$REPORT_FILE" << EOF
BeagleBoard Imager E2E Test Report
===================================
Platform: ${PLATFORM}
Date: $(date)
Host: $(hostname)
Rust Version: $(rustc --version)

EOF
fi

# Track overall success
OVERALL_SUCCESS=0

# Run SD card tests
if [ "$RUN_SD" = true ]; then
    if run_tests "sd" "SD Card"; then
        :
    else
        OVERALL_SUCCESS=1
    fi
fi

# Run BCF tests
if [ "$RUN_BCF" = true ]; then
    if run_tests "bcf,bcf_msp430" "BeagleConnect Freedom"; then
        :
    else
        OVERALL_SUCCESS=1
    fi
fi

# Run DFU tests
if [ "$RUN_DFU" = true ]; then
    if run_tests "dfu" "DFU"; then
        :
    else
        OVERALL_SUCCESS=1
    fi
fi

# Generate summary
echo -e "\n${YELLOW}============================================${NC}"
echo -e "${YELLOW}Test Summary${NC}"
echo -e "${YELLOW}============================================${NC}"
echo -e "Platform: ${PLATFORM}"

if [ $OVERALL_SUCCESS -eq 0 ]; then
    echo -e "${GREEN}All tests PASSED!${NC}"
else
    echo -e "${RED}Some tests FAILED!${NC}"
fi

if [ "$GENERATE_REPORT" = true ]; then
    echo -e "\nDetailed report saved to: ${REPORT_FILE}"
    cat >> "$REPORT_FILE" << EOF

============================================
Test Summary
============================================
Overall Status: $([ $OVERALL_SUCCESS -eq 0 ] && echo "PASSED" || echo "FAILED")
EOF
fi

exit $OVERALL_SUCCESS

