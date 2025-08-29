use criterion::{criterion_group, criterion_main, Criterion, black_box};
use std::time::Duration;
use tfhe::core_crypto::prelude::*;
use tfhe::core_crypto::fft_impl::fft64::crypto::ggsw::add_external_product_assign_scratch;
use tfhe::core_crypto::algorithms::glwe_encryption::encrypt_glwe_ciphertext;
use tfhe::core_crypto::algorithms::ggsw_encryption::encrypt_constant_ggsw_ciphertext;

fn external_product_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("external_product");
    group.measurement_time(Duration::from_secs(30));
    group.sample_size(10);

    // Parameters: polynomial degree 1024, GLWE dimension 1
    let glwe_dimension = GlweDimension(1);
    let polynomial_size = PolynomialSize(1024);
    let glwe_size = glwe_dimension.to_glwe_size();
    let decomp_base_log = DecompositionBaseLog(7);
    let decomp_level_count = DecompositionLevelCount(3);
    let ciphertext_modulus = CiphertextModulus::new_native();

    println!("=== External Product Assign Benchmark ===");
    println!("GLWE Dimension: {}", glwe_dimension.0);
    println!("Polynomial Size: {}", polynomial_size.0);
    println!("GLWE Size: {}", glwe_size.0);
    println!("Decomposition Base Log: {}", decomp_base_log.0);
    println!("Decomposition Level Count: {}", decomp_level_count.0);
    println!("==========================================");

    // Create PRNG
    let mut seeder = new_seeder();
    let seeder = seeder.as_mut();
    let mut encryption_generator = 
        EncryptionRandomGenerator::<DefaultRandomGenerator>::new(seeder.seed(), seeder);
    let mut secret_generator = 
        SecretRandomGenerator::<DefaultRandomGenerator>::new(seeder.seed());

    // Generate GLWE secret key
    let glwe_secret_key: GlweSecretKeyOwned<u64> =
        allocate_and_generate_new_binary_glwe_secret_key(
            glwe_dimension,
            polynomial_size,
            &mut secret_generator,
        );

    // Create test GLWE ciphertext (input) - manually create and encrypt
    let input_plaintext = PlaintextList::new(42u64, PlaintextCount(polynomial_size.0));
    let mut input_glwe = GlweCiphertext::new(
        0u64,
        glwe_size,
        polynomial_size,
        ciphertext_modulus,
    );
    encrypt_glwe_ciphertext(
        &glwe_secret_key,
        &mut input_glwe,
        &input_plaintext,
        DynamicDistribution::new_gaussian_from_std_dev(StandardDev(0.00000001)),
        &mut encryption_generator,
    );

    // Create output GLWE ciphertext (will be modified by external product)
    let output_plaintext = PlaintextList::new(1u64, PlaintextCount(polynomial_size.0));
    let mut output_glwe = GlweCiphertext::new(
        0u64,
        glwe_size,
        polynomial_size,
        ciphertext_modulus,
    );
    encrypt_glwe_ciphertext(
        &glwe_secret_key,
        &mut output_glwe,
        &output_plaintext,
        DynamicDistribution::new_gaussian_from_std_dev(StandardDev(0.00000001)),
        &mut encryption_generator,
    );

    // Create standard GGSW ciphertext - manually create and encrypt
    let mut std_ggsw = GgswCiphertext::new(
        0u64,
        glwe_size,
        polynomial_size,
        decomp_base_log,
        decomp_level_count,
        ciphertext_modulus,
    );
    encrypt_constant_ggsw_ciphertext(
        &glwe_secret_key,
        &mut std_ggsw,
        Cleartext(2u64),
        DynamicDistribution::new_gaussian_from_std_dev(StandardDev(0.00000001)),
        &mut encryption_generator,
    );

    // Convert GGSW to Fourier domain
    let mut fourier_ggsw = FourierGgswCiphertext::new(
        glwe_size,
        polynomial_size,
        decomp_base_log,
        decomp_level_count,
    );

    let fft = Fft::new(polynomial_size);
    let fft_view = fft.as_view();

    // Fill Fourier GGSW with forward FFT
    let mut buffers = ComputationBuffers::new();
    buffers.resize(
        fft_view.forward_scratch()
            .unwrap()
            .unaligned_bytes_required(),
    );

    println!("Converting GGSW to Fourier domain...");
    fourier_ggsw
        .as_mut_view()
        .fill_with_forward_fourier(std_ggsw.as_view(), fft_view, buffers.stack());

    // Setup buffers for external product
    let mut ep_buffers = ComputationBuffers::new();
    ep_buffers.resize(
        add_external_product_assign_scratch::<u64>(
            glwe_size,
            polynomial_size,
            fft_view,
        )
        .unwrap()
        .unaligned_bytes_required(),
    );

    println!("Starting external product benchmark...");

    // Run the benchmark
    group.bench_function("add_external_product_assign_1024", |b| {
        b.iter(|| {
            let mut output_copy = output_glwe.clone();
            
            tfhe::core_crypto::fft_impl::fft64::crypto::ggsw::add_external_product_assign(
                output_copy.as_mut_view(),
                fourier_ggsw.as_view(),
                input_glwe.as_view(),
                fft_view,
                ep_buffers.stack(),
            );
            
            black_box(&output_copy);
        });
    });

    println!("Benchmark completed!");
    group.finish();
}

criterion_group!(external_product_group, external_product_benchmark);
criterion_main!(external_product_group);