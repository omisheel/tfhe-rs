# Blind Rotate Benchmark

This directory contains benchmarks specifically for the `blind_rotate_assign` function, which is the core operation in TFHE's programmable bootstrapping.

## Overview

The `blind_rotate_assign` function is one of the most computationally expensive operations in fully homomorphic encryption (FHE). It performs "blind rotation" of a lookup table based on an encrypted index, enabling homomorphic evaluation of arbitrary functions while reducing noise.

## Benchmarks Included

### 1. `blind_rotate_test.rs`
A simple, fast benchmark that tests basic functionality with small parameters:
- **Purpose**: Quick validation and basic performance measurement
- **Parameters**: Small (LWE dim: 512, GLWE dim: 2, poly size: 512)
- **Runtime**: ~30 seconds
- **Use case**: Development and CI testing

### 2. `blind_rotate_bench.rs` 
Comprehensive benchmark testing multiple parameter sets and scenarios:
- **Purpose**: Full performance evaluation across different security levels
- **Parameters**: Multiple sets (32-bit and 64-bit variants)
- **Modes**: Both latency and throughput testing
- **Runtime**: Several minutes
- **Use case**: Performance analysis and optimization

## Running the Benchmarks

### Quick Test
```bash
# Run the simple test (fast)
cd tfhe-rs/tfhe-benchmark
cargo bench --bench blind-rotate-test --features="shortint,internal-keycache"
```

### Full Benchmark
```bash
# Run comprehensive benchmark
cargo bench --bench blind-rotate-bench --features="shortint,internal-keycache"
```

### Using the Convenience Script
```bash
# Run both benchmarks with proper environment setup
./run_blind_rotate_bench.sh
```

## Configuration Options

### Benchmark Type
Set via environment variable `TFHE_RS_BENCH_TYPE`:
- `latency` (default): Measures single operation time
- `throughput`: Measures operations per second with parallelization

### Parameter Sets
Set via environment variable `TFHE_RS_BENCH_PARAMS_SET`:
- `default`: Standard security parameters
- `small`: Faster, less secure parameters for testing

### Examples
```bash
# Throughput benchmark
TFHE_RS_BENCH_TYPE=throughput cargo bench --bench blind-rotate-bench --features="shortint,internal-keycache"

# Small parameters for faster testing
TFHE_RS_BENCH_PARAMS_SET=small cargo bench --bench blind-rotate-test --features="shortint,internal-keycache"
```

## What Gets Measured

### Core Operations
1. **Lookup Table Setup**: Creating and initializing the accumulator (GLWE ciphertext)
2. **Modulus Switching**: Converting LWE ciphertext to blind rotation format
3. **Blind Rotation**: The main algorithm that rotates the LUT
4. **Memory Management**: Buffer allocation and FFT scratch space

### Key Metrics
- **Latency**: Time per single blind rotation operation
- **Throughput**: Operations per second (parallel execution)
- **Memory Usage**: Peak memory consumption
- **FFT Performance**: Fourier domain computation efficiency

## Understanding the Results

### Typical Performance
On a modern CPU, expect:
- **Small parameters**: ~1-5ms per operation
- **Standard parameters**: ~10-50ms per operation  
- **Large parameters**: ~100-500ms per operation

### Performance Factors
1. **Polynomial Size**: Larger sizes = more FFT work
2. **Decomposition Levels**: More levels = more GGSW multiplications
3. **LWE Dimension**: Affects number of CMUX operations
4. **FFT Implementation**: AVX-512 vs standard SIMD

### Optimization Opportunities
- **Memory Layout**: Aligned buffers for SIMD efficiency
- **FFT Strategy**: Batch vs individual transforms
- **Parallelization**: Multi-threading blind rotations
- **Hardware**: AVX-512 support for faster complex arithmetic

## Benchmark Architecture

### Memory Management
- Uses `ComputationBuffers` for scratch space allocation
- Preallocates FFT buffers to avoid runtime allocation
- Memory-aligned buffers for optimal SIMD performance

### Test Data Generation
- Random secret keys for realistic encryption
- Identity function lookup tables for easy verification
- Multiple ciphertext inputs for throughput testing

### Result Output
- JSON format for automated analysis
- Compatible with TFHE-rs benchmarking infrastructure
- Includes parameter metadata and system information

## Troubleshooting

### Common Issues
1. **Out of Memory**: Reduce parameter sizes or batch count
2. **Slow Performance**: Enable `nightly-avx512` feature if supported
3. **Compilation Errors**: Ensure all required features are enabled

### Debug Mode
```bash
# Run with debug output
RUST_LOG=debug cargo bench --bench blind-rotate-test --features="shortint,internal-keycache"
```

### Verification
The benchmark includes correctness checks to ensure the blind rotation produces expected results.

## Related Files

- `tfhe/src/core_crypto/fft_impl/fft64/crypto/bootstrap.rs` - Main implementation
- `tfhe/src/core_crypto/fft_impl/fft64/crypto/ggsw.rs` - External product operations
- `tfhe-benchmark/benches/core_crypto/pbs_bench.rs` - Full PBS benchmark
- `tfhe-benchmark/benches/core_crypto/ks_pbs_bench.rs` - Key-switch + PBS benchmark

## Contributing

When modifying the blind rotate implementation:
1. Run these benchmarks to measure performance impact
2. Compare before/after results using the same parameters
3. Consider adding new test cases for edge conditions
4. Update benchmark parameters if security levels change

## Performance Analysis

### Profiling
For detailed analysis, use:
```bash
# CPU profiling
cargo bench --bench blind-rotate-bench --features="shortint,internal-keycache" -- --profile-time=30

# Memory profiling  
valgrind --tool=massif cargo bench --bench blind-rotate-test --features="shortint,internal-keycache"
```

### Flamegraphs
```bash
# Generate flamegraph (requires flamegraph crate)
cargo flamegraph --bench blind-rotate-test --features="shortint,internal-keycache"
```
