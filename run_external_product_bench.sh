#!/bin/bash

# Script to run the add_external_product_assign benchmark
# This focuses on the external product operation used in GGSW ciphertext multiplication
# Parameters:
# - Polynomial Size: 1024
# - GLWE Dimension: 1

set -e

echo "========================================"
echo "External Product Assign Benchmark"
echo "========================================"
echo ""
echo "Parameters:"
echo "  Polynomial Size: 1024"
echo "  GLWE Dimension: 1"
echo "  Decomposition Base Log: 7"
echo "  Decomposition Level Count: 3"
echo ""
echo "This benchmark measures the performance of the"
echo "add_external_product_assign function, which computes:"
echo "  output += GGSW ⊠ GLWE"
echo ""
echo "This is a core operation in blind rotation and"
echo "homomorphic multiplication in TFHE."
echo ""

# Change to the tfhe-benchmark directory
cd "$(dirname "$0")/tfhe-benchmark"

echo "Building benchmark..."
cargo build --release --bench external-product-bench --features="shortint,internal-keycache"

echo ""
echo "Running benchmark..."
echo "This will take approximately 5-10 minutes..."
echo ""

# Run the benchmark with timing
time cargo bench --bench external-product-bench --features="shortint,internal-keycache"

echo ""
echo "========================================"
echo "Benchmark completed!"
echo "========================================"
echo ""
echo "The results show the time for a single add_external_product_assign"
echo "operation with polynomial size 1024 and GLWE dimension 1."
echo ""
echo "Expected performance on modern hardware:"
echo "  - Typical range: 1-5ms per operation"
echo "  - With AVX-512: 0.5-3ms per operation"
echo ""
echo "This operation is called multiple times during blind rotation:"
echo "  - Once per LWE dimension element (630 times for TFHE_LIB_PARAMETERS)"
echo "  - Once per decomposition level (3 times for TFHE_LIB_PARAMETERS)"
echo "  - Total: ~1890 external products per blind rotation"
echo ""
echo "To run with AVX-512 optimization (if supported):"
echo "  cargo bench --bench external-product-bench --features=\"shortint,internal-keycache,nightly-avx512\""