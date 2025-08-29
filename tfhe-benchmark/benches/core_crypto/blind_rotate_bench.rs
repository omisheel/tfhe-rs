use benchmark::params::shortint_params::shortint_params_keycache::benchmark_parameters;
use benchmark::utilities::{
    get_bench_type, throughput_num_threads, write_to_json, BenchmarkType, CryptoParametersRecord,
    OperatorType,
};
use criterion::{black_box, Criterion, Throughput};
use rayon::prelude::*;
use serde::Serialize;

use tfhe::core_crypto::prelude::*;
use tfhe::core_crypto::fft_impl::fft64::crypto::bootstrap::{
    blind_rotate_assign_scratch, fill_with_forward_fourier_scratch,
};

fn mem_optimized_blind_rotate<Scalar: UnsignedTorus + CastInto<usize> + CastFrom<usize> + Serialize>(
    c: &mut Criterion,
    parameters: &[(String, CryptoParametersRecord<Scalar>)],
) {
    let bench_name = "core_crypto::blind_rotate_assign_mem_optimized";
    let mut bench_group = c.benchmark_group(bench_name);
    bench_group
        .sample_size(10)
        .measurement_time(std::time::Duration::from_secs(30));

    // Create the PRNG
    let mut seeder = new_seeder();
    let seeder = seeder.as_mut();
    let mut encryption_generator =
        EncryptionRandomGenerator::<DefaultRandomGenerator>::new(seeder.seed(), seeder);
    let mut secret_generator = SecretRandomGenerator::<DefaultRandomGenerator>::new(seeder.seed());

    for (name, params) in parameters.iter() {
        // Create the LweSecretKey
        let input_lwe_secret_key = allocate_and_generate_new_binary_lwe_secret_key(
            params.lwe_dimension.unwrap(),
            &mut secret_generator,
        );
        let _output_glwe_secret_key: GlweSecretKeyOwned<Scalar> =
            allocate_and_generate_new_binary_glwe_secret_key(
                params.glwe_dimension.unwrap(),
                params.polynomial_size.unwrap(),
                &mut secret_generator,
            );

        // Create the empty bootstrapping key in the Fourier domain
        let mut fourier_bsk = FourierLweBootstrapKey::new(
            params.lwe_dimension.unwrap(),
            params.glwe_dimension.unwrap().to_glwe_size(),
            params.polynomial_size.unwrap(),
            params.pbs_base_log.unwrap(),
            params.pbs_level.unwrap(),
        );

        // Create the standard bootstrapping key for conversion
        let std_bsk = LweBootstrapKey::new(
            Scalar::ZERO,
            params.glwe_dimension.unwrap().to_glwe_size(),
            params.polynomial_size.unwrap(),
            params.pbs_base_log.unwrap(),
            params.pbs_level.unwrap(),
            params.lwe_dimension.unwrap(),
            params.ciphertext_modulus.unwrap(),
        );

        let fft = Fft::new(fourier_bsk.polynomial_size());
        let fft = fft.as_view();

        // Fill the Fourier BSK with forward FFT
        let mut buffers = ComputationBuffers::new();
        buffers.resize(
            fill_with_forward_fourier_scratch(fft)
                .unwrap()
                .unaligned_bytes_required(),
        );
        fourier_bsk
            .as_mut_view()
            .fill_with_forward_fourier(std_bsk.as_view(), fft, buffers.stack());

        let bench_id;

        match get_bench_type() {
            BenchmarkType::Latency => {
                // Allocate a new LweCiphertext and encrypt our plaintext
                let lwe_ciphertext_in: LweCiphertextOwned<Scalar> =
                    allocate_and_encrypt_new_lwe_ciphertext(
                        &input_lwe_secret_key,
                        Plaintext(Scalar::ONE),
                        params.lwe_noise_distribution.unwrap(),
                        params.ciphertext_modulus.unwrap(),
                        &mut encryption_generator,
                    );

                // Create the lookup table (accumulator)
                let mut accumulator = GlweCiphertext::new(
                    Scalar::ZERO,
                    params.glwe_dimension.unwrap().to_glwe_size(),
                    params.polynomial_size.unwrap(),
                    params.ciphertext_modulus.unwrap(),
                );

                // Create a simple test lookup table (identity function)
                let poly_size = params.polynomial_size.unwrap().0;
                let delta: Scalar = (Scalar::ONE << (Scalar::BITS - 1))
                    / Scalar::cast_from(poly_size);
                accumulator
                    .as_mut_polynomial_list()
                    .iter_mut()
                    .enumerate()
                    .for_each(|(poly_index, mut polynomial)| {
                        if poly_index == 0 {
                            // First polynomial gets the lookup table
                            polynomial
                                .iter_mut()
                                .enumerate()
                                .for_each(|(coeff_index, coeff)| {
                                    *coeff = Scalar::cast_from(coeff_index) * delta;
                                });
                        }
                        // Other polynomials remain zero
                    });

                // Prepare modulus switched LWE ciphertext for blind rotation
                let log_modulus = accumulator
                    .polynomial_size()
                    .to_blind_rotation_input_modulus_log();
                let msed_lwe = lwe_ciphertext_modulus_switch(lwe_ciphertext_in.as_view(), log_modulus);

                let mut buffers = ComputationBuffers::new();

                buffers.resize(
                    blind_rotate_assign_scratch::<Scalar>(
                        fourier_bsk.glwe_size(),
                        fourier_bsk.polynomial_size(),
                        fft,
                    )
                    .unwrap()
                    .unaligned_bytes_required(),
                );

                bench_id = format!("{bench_name}::{name}");

                bench_group.bench_function(&bench_id, |b| {
                    b.iter(|| {
                        let mut lut_copy = accumulator.clone();
                        fourier_bsk.as_view().blind_rotate_assign(
                            lut_copy.as_mut_view(),
                            &msed_lwe,
                            fft,
                            buffers.stack(),
                        );
                        black_box(&mut lut_copy);
                    })
                });
            }
            BenchmarkType::Throughput => {
                bench_id = format!("{bench_name}::throughput::{name}");
                let blocks: usize = 1;
                let elements = throughput_num_threads(blocks, 1);
                bench_group.throughput(Throughput::Elements(elements));
                bench_group.bench_function(&bench_id, |b| {
                    // Pre-create all the test data
                    let input_cts: Vec<LweCiphertextOwned<Scalar>> = (0..elements)
                        .map(|_| {
                            allocate_and_encrypt_new_lwe_ciphertext(
                                &input_lwe_secret_key,
                                Plaintext(Scalar::ONE),
                                params.lwe_noise_distribution.unwrap(),
                                params.ciphertext_modulus.unwrap(),
                                &mut encryption_generator,
                            )
                        })
                        .collect();

                    let log_modulus = params.polynomial_size.unwrap()
                        .to_blind_rotation_input_modulus_log();
                    
                    b.iter(|| {
                        let mut accumulators: Vec<GlweCiphertextOwned<Scalar>> = (0..elements)
                            .map(|_| {
                                let mut accumulator = GlweCiphertext::new(
                                    Scalar::ZERO,
                                    params.glwe_dimension.unwrap().to_glwe_size(),
                                    params.polynomial_size.unwrap(),
                                    params.ciphertext_modulus.unwrap(),
                                );

                                // Create a simple test lookup table (identity function)
                                let poly_size = params.polynomial_size.unwrap().0;
                                let delta: Scalar = (Scalar::ONE << (Scalar::BITS - 1))
                                    / Scalar::cast_from(poly_size);
                                accumulator
                                    .as_mut_polynomial_list()
                                    .iter_mut()
                                    .enumerate()
                                    .for_each(|(poly_index, mut polynomial)| {
                                        if poly_index == 0 {
                                            // First polynomial gets the lookup table
                                            polynomial
                                                .iter_mut()
                                                .enumerate()
                                                .for_each(|(coeff_index, coeff)| {
                                                    *coeff = Scalar::cast_from(coeff_index) * delta;
                                                });
                                        }
                                        // Other polynomials remain zero
                                    });

                                accumulator
                            })
                            .collect();

                        accumulators
                            .par_iter_mut()
                            .zip(input_cts.par_iter())
                            .for_each(|(accumulator, input_ct)| {
                                let msed_lwe = lwe_ciphertext_modulus_switch(input_ct.as_view(), log_modulus);
                                let mut buffers = ComputationBuffers::new();
                                buffers.resize(
                                    blind_rotate_assign_scratch::<Scalar>(
                                        fourier_bsk.glwe_size(),
                                        fourier_bsk.polynomial_size(),
                                        fft,
                                    )
                                    .unwrap()
                                    .unaligned_bytes_required(),
                                );

                                fourier_bsk.as_view().blind_rotate_assign(
                                    accumulator.as_mut_view(),
                                    &msed_lwe,
                                    fft,
                                    buffers.stack(),
                                );
                            });
                        black_box(accumulators);
                    });
                });
            }
        }

        let modulus_value = if params.ciphertext_modulus.unwrap().is_native_modulus() {
            64u32  // For native modulus, use 64 bits
        } else {
            params.ciphertext_modulus.unwrap().get_custom_modulus() as u32
        };
        
        write_to_json(
            &bench_id,
            *params,
            name,
            "blind_rotate_assign",
            &OperatorType::Atomic,
            modulus_value,
            vec![modulus_value],
        );
    }

    bench_group.finish()
}

use criterion::criterion_group;
use criterion::criterion_main;

fn bench_blind_rotate_64_bit(c: &mut Criterion) {
    let bench_params = benchmark_parameters();
    mem_optimized_blind_rotate(c, &bench_params)
}

criterion_group!(
    blind_rotate_benches,
    bench_blind_rotate_64_bit
);

criterion_main!(blind_rotate_benches);