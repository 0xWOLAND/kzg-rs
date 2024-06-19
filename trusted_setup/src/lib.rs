extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;

const TRUSTED_SETUP_FILE: &str = include_str!("trusted_setup.txt");

fn hex_to_bytes(hex_str: &str) -> Vec<u8> {
    let trimmed_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    hex::decode(trimmed_str).unwrap()
}

#[proc_macro]
pub fn include_trusted_setup_file(_input: TokenStream) -> TokenStream {
    let trusted_setup_file: Vec<String> =
        TRUSTED_SETUP_FILE.lines().map(|x| x.to_string()).collect();

    let num_g1_points = trusted_setup_file[0].parse::<usize>().unwrap();
    let num_g2_points = trusted_setup_file[1].parse::<usize>().unwrap();
    let g1_points_idx = num_g1_points + 2;
    let g2_points_idx = g1_points_idx + num_g2_points;

    let g1_points: Vec<String> = trusted_setup_file[2..g1_points_idx].to_vec();
    let g2_points: Vec<String> = trusted_setup_file[g1_points_idx..g2_points_idx].to_vec();

    // Generate Rust code for the trusted setup data
    let g1_points_code = g1_points.iter().map(|p| {
        let bytes = hex_to_bytes(p);
        let array: [u8; 48] = bytes.try_into().unwrap();
        quote! { [#(#array),*] }
    });

    let g2_points_code = g2_points.iter().map(|p| {
        let bytes = hex_to_bytes(p);
        let array: [u8; 96] = bytes.try_into().unwrap();
        quote! { [#(#array),*] }
    });

    let expanded = quote! {
        pub fn load_trusted_setup_file() -> Result<KzgSettings, KzgError> {
            const NUM_G1_POINTS: usize = #num_g1_points;
            const NUM_G2_POINTS: usize = #num_g2_points;
            const G1_POINTS: [[u8; 48]; #num_g1_points] = [
                #(#g1_points_code),*
            ];
            const G2_POINTS: [[u8; 96]; #num_g2_points] = [
                #(#g2_points_code),*
            ];

            let mut kzg_settings = KzgSettings::default();

            let mut max_scale = 0;
            while (1 << max_scale) < NUM_G1_POINTS {
                max_scale += 1;
            }
            kzg_settings.max_width = 1 << max_scale;

            G1_POINTS.iter().enumerate().for_each(|(i, bytes)| {
                println!("cycle-tracker-start: g1-points-{:?}", i);
                let g1_affine = G1Affine::from_compressed_unchecked(bytes)
                    .expect("load_trusted_setup Invalid g1 bytes");
                kzg_settings.g1_values.push(g1_affine);
                println!("cycle-tracker-end: g1-points-{:?}", i);
            });
            G2_POINTS.iter().enumerate().for_each(|(i,bytes)| {
                println!("cycle-tracker-start: g2-points-{:?}", i);
                let g2_affine = G2Affine::from_compressed_unchecked(bytes)
                    .expect("load_trusted_setup Invalid g2 bytes");
                kzg_settings.g2_values.push(g2_affine);
                println!("cycle-tracker-end: g2-points-{:?}", i);
            });

            let _ = is_trusted_setup_in_lagrange_form(&kzg_settings);

            let bit_reversed_permutation = bit_reversal_permutation(kzg_settings.g1_values)?;
            kzg_settings.g1_values = bit_reversed_permutation;

            Ok(kzg_settings)
        }
    };

    TokenStream::from(expanded)
}
