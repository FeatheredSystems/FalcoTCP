#!/bin/bash

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "========================================"
echo "Running Cargo Tests"
echo "========================================"
echo ""

# Clean
echo -e "Cleaning..."
cargo clean

# Track error types
declare -A error_types
linker_errors=0
compile_errors=0

# Array of test configurations
declare -a tests=(
    "Default:--features default"
    "Complete Compression - Performance Bias:--features complete-compression,heuristics,heubias-performance,encryption"
    "Complete Compression - Balance Bias:--features complete-compression,heuristics,heubias-balance,encryption"
    "Complete Compression - Ratio Bias:--features complete-compression,heuristics,heubias-ratio,encryption"
    "LZMA Only:--no-default-features --features LZMA,encryption,heuristics,heubias-balance,falco-client,falco-server"
    "ZSTD Only:--no-default-features --features ZSTD,encryption,heuristics,heubias-balance,falco-client,falco-server"
    "GZIP Only:--no-default-features --features GZIP,encryption,heuristics,heubias-balance,falco-client,falco-server"
    "LZ4 Only:--no-default-features --features LZ4,encryption,heuristics,heubias-balance,falco-client,falco-server"
    "No Compression - Encryption Only:--no-default-features --features encryption,falco-client,falco-server"
    "Complete Compression - No Encryption:--no-default-features --features complete-compression,heuristics,heubias-balance,falco-client,falco-server"
    "Client Only:--no-default-features --features falco-client,encryption,complete-compression,heuristics,heubias-balance"
    "Server Only:--no-default-features --features falco-server,encryption,complete-compression,heuristics,heubias-balance"
    "Minimal - No Features:--no-default-features"
    "With Tokio Runtime:--features default,tokio-runtime"
    "LZMA + ZSTD:--no-default-features --features LZMA,ZSTD,encryption,heuristics,heubias-balance,falco-client,falco-server"
    "GZIP + LZ4:--no-default-features --features GZIP,LZ4,encryption,heuristics,heubias-balance,falco-client,falco-server"
    "Heuristics - Performance (No Compression):--no-default-features --features heuristics,heubias-performance,encryption,falco-client,falco-server"
    "TLS + Complete Compression:--features tls,complete-compression,heuristics,heubias-balance,falco-client,falco-server"
    "Tokio TLS + Complete Compression:--features tokio-tls,complete-compression,heuristics,heubias-balance,falco-client,falco-server"
)

# Counter for results
passed=0
failed=0
failed_tests=()

# Run each test
for test in "${tests[@]}"; do
    IFS=':' read -r name flags <<< "$test"
    
    echo -e "${YELLOW}Testing: $name${NC}"
    echo "Flags: $flags"
    echo ""
    
    if cargo test $flags --lib 2>&1; then
        echo -e "${GREEN}✓ PASSED: $name${NC}"
        ((passed++))
    else
        echo -e "${RED}✗ FAILED: $name${NC}"
        ((failed++))
        failed_tests+=("$name")
    fi
    
    echo ""
    echo "----------------------------------------"
    echo ""
done


# Additional checks
echo -e "${YELLOW}Running additional checks...${NC}"
echo ""

echo "Checking all features compile..."
if cargo check --all-features 2>&1; then
    echo -e "${GREEN}✓ PASSED: All features check${NC}"
    ((passed++))
else
    echo -e "${RED}✗ FAILED: All features check${NC}"
    ((failed++))
    failed_tests+=("All features check")
fi
echo ""

echo "Checking no-default-features compile..."
if cargo check --no-default-features 2>&1; then
    echo -e "${GREEN}✓ PASSED: No default features check${NC}"
    ((passed++))
else
    echo -e "${RED}✗ FAILED: No default features check${NC}"
    ((failed++))
    failed_tests+=("No default features check")
fi
echo ""

# Summary
echo "========================================"
echo "Test Summary"
echo "========================================"
echo -e "${GREEN}Passed: $passed${NC}"
echo -e "${RED}Failed: $failed${NC}"
echo ""

if [ $failed -gt 0 ]; then
    echo -e "${RED}Failed tests:${NC}"
    for test in "${failed_tests[@]}"; do
        echo "  - $test"
    done
    exit 1
else
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
fi
