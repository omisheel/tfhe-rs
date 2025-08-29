#!/bin/bash

# Script to run the TFHE_LIB_PARAMETERS blind rotate benchmark
# This focuses specifically on the original TFHE library parameters:
# - LWE Dimension: 630
# - GLWE Dimension: 1  
# - Polynomial Size: 1024
# - Decomposition Base Log: 7
# - Decomposition Level Count: 3

set -e

echo "========================================"
echo "TFHE_LIB_PARAMETERS Blind Rotate Benchmark"
echo "========================================"
echo ""
echo "Parameters:"
echo "  LWE Dimension: 630"
echo "  GLWE Dimension: 1"
echo "  Polynomial Size: 1024"
echo "  Decomposition Base Log: 7"
echo "  Decomposition Level Count: 3"
echo ""
echo "This benchmark measures the performance of the"
echo "blind_rotate_assign function with the original"
echo "TFHE library parameters."
echo ""

# Change to the tfhe-benchmark directory
cd "$(dirname "$0")/tfhe-benchmark"

echo "Building benchmark..."
cargo build --release --bench blind-rotate-tfhe-lib --features="shortint,internal-keycache"

echo ""
echo "Running benchmark..."
echo "This will take approximately 5-10 minutes..."
echo ""

# Run the benchmark with timing
time cargo bench --bench blind-rotate-tfhe-lib --features="shortint,internal-keycache"

echo ""
echo "========================================"
echo "Benchmark completed!"
echo "========================================"
echo ""
echo "The results show the time for a single blind_rotate_assign"
echo "operation using TFHE_LIB_PARAMETERS."
echo ""
echo "Expected performance on modern hardware:"
echo "  - Typical range: 10-50ms per operation"
echo "  - With AVX-512: 5-25ms per operation"
echo ""
echo "To run with AVX-512 optimization (if supported):"
echo "  cargo bench --bench blind-rotate-tfhe-lib --features=\"shortint,internal-keycache,nightly-avx512\""