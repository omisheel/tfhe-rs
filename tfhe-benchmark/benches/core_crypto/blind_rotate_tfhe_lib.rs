use criterion::{criterion_group, criterion_main, Criterion, black_box};
use std::time::Duration;
use tfhe::core_crypto::prelude::*;
use tfhe::core_crypto::fft_impl::fft64::crypto::bootstrap::blind_rotate_assign_scratch;

fn tfhe_lib_blind_rotate_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("tfhe_lib_blind_rotate");
    group.measurement_time(Duration::from_secs(30));
    group.sample_size(10);

    // TFHE_LIB_PARAMETERS equivalent
    let lwe_dimension = LweDimension(630);
    let glwe_dimension = GlweDimension(1);
    let polynomial_size = PolynomialSize(1024);
    let decomp_base_log = DecompositionBaseLog(7);
    let decomp_level_count = DecompositionLevelCount(3);
    let lwe_std_dev = StandardDev(0.000030517578125);
    let glwe_std_dev = StandardDev(0.00000002980232238769531);
    let ciphertext_modulus = CiphertextModulus::new_native();

    println!("=== TFHE_LIB_PARAMETERS Blind Rotate Benchmark ===");
    println!("LWE Dimension: {}", lwe_dimension.0);
    println!("GLWE Dimension: {}", glwe_dimension.0);
    println!("Polynomial Size: {}", polynomial_size.0);
    println!("Decomposition Base Log: {}", decomp_base_log.0);
    println!("Decomposition Level Count: {}", decomp_level_count.0);
    println!("===============================================");

    // Create PRNG
    let mut seeder = new_seeder();
    let seeder = seeder.as_mut();
    let mut encryption_generator = 
        EncryptionRandomGenerator::<DefaultRandomGenerator>::new(seeder.seed(), seeder);
    let mut secret_generator = 
        SecretRandomGenerator::<DefaultRandomGenerator>::new(seeder.seed());

    // Generate keys
    let input_lwe_secret_key = allocate_and_generate_new_binary_lwe_secret_key(
        lwe_dimension,
        &mut secret_generator,
    );
    
    let output_glwe_secret_key: GlweSecretKeyOwned<u64> =
        allocate_and_generate_new_binary_glwe_secret_key(
            glwe_dimension,
            polynomial_size,
            &mut secret_generator,
        );

    // Create empty bootstrapping key
    let mut fourier_bsk = FourierLweBootstrapKey::new(
        lwe_dimension,
        glwe_dimension.to_glwe_size(),
        polynomial_size,
        decomp_base_log,
        decomp_level_count,
    );

    // Create standard bootstrapping key for FFT conversion
    let std_bsk = LweBootstrapKey::new(
        0u64,
        glwe_dimension.to_glwe_size(),
        polynomial_size,
        decomp_base_log,
        decomp_level_count,
        lwe_dimension,
        ciphertext_modulus,
    );

    let fft = Fft::new(polynomial_size);
    let fft_view = fft.as_view();

    // Fill Fourier BSK with forward FFT
    let mut buffers = ComputationBuffers::new();
    buffers.resize(
        fft_view.forward_scratch()
            .unwrap()
            .unaligned_bytes_required(),
    );
    
    println!("Converting bootstrapping key to Fourier domain...");
    fourier_bsk
        .as_mut_view()
        .fill_with_forward_fourier(std_bsk.as_view(), fft_view, buffers.stack());

    // Create test LWE ciphertext (encrypting value 1)
    let lwe_ct = allocate_and_encrypt_new_lwe_ciphertext(
        &input_lwe_secret_key,
        Plaintext(1u64 << 60), // High bit set for clear signal
        DynamicDistribution::new_gaussian_from_std_dev(lwe_std_dev),
        ciphertext_modulus,
        &mut encryption_generator,
    );

    // Create lookup table (accumulator) - identity function
    let mut accumulator = GlweCiphertext::new(
        0u64,
        glwe_dimension.to_glwe_size(),
        polynomial_size,
        ciphertext_modulus,
    );

    // Initialize with identity LUT: LUT[i] = i * delta
    let delta = 1u64 << (64 - 1 - polynomial_size.log2().0);
    accumulator
        .as_mut_polynomial_list()
        .iter_mut()
        .enumerate()
        .for_each(|(poly_idx, mut poly)| {
            if poly_idx == 0 {
                // Only first polynomial gets the lookup table
                poly.iter_mut()
                    .enumerate()
                    .for_each(|(coeff_idx, coeff)| {
                        *coeff = (coeff_idx as u64).wrapping_mul(delta);
                    });
            }
            // Other polynomials remain zero (there's only one for GLWE dim 1)
        });

    // Prepare modulus switched LWE ciphertext for blind rotation
    let log_modulus = polynomial_size.to_blind_rotation_input_modulus_log();
    let msed_lwe = lwe_ciphertext_modulus_switch(lwe_ct.as_view(), log_modulus);

    // Setup buffers for blind rotation
    let mut br_buffers = ComputationBuffers::new();
    br_buffers.resize(
        blind_rotate_assign_scratch::<u64>(
            glwe_dimension.to_glwe_size(),
            polynomial_size,
            fft_view,
        )
        .unwrap()
        .unaligned_bytes_required(),
    );

    println!("Starting blind rotate benchmark...");

    // Run the benchmark
    group.bench_function("blind_rotate_assign_tfhe_lib", |b| {
        b.iter(|| {
            let mut lut_copy = accumulator.clone();
            fourier_bsk.as_view().blind_rotate_assign(
                lut_copy.as_mut_view(),
                &msed_lwe,
                fft_view,
                br_buffers.stack(),
            );
            black_box(&lut_copy);
        });
    });

    println!("Benchmark completed!");
    group.finish();
}

criterion_group!(tfhe_lib_blind_rotate_group, tfhe_lib_blind_rotate_benchmark);
criterion_main!(tfhe_lib_blind_rotate_group);