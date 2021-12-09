//! Autogenerated weights for pallet_grandpa
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2021-11-26, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// ./target/release/zeitgeist
// benchmark
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_grandpa
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --template=./misc/frame_weight_template.hbs
// --output=./runtime/src/weights/

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions for pallet_grandpa (automatically generated)
pub struct WeightInfo<T>(PhantomData<T>);

impl<T: frame_system::Config> pallet_grandpa::weights::WeightInfo for WeightInfo<T> {

	fn check_equivocation_proof(x: u32, ) -> Weight {
		(206_093_000 as Weight)
			// Standard Error: 82_000
			.saturating_add((33_636_000 as Weight).saturating_mul(x as Weight))
	}
	// Storage: Grandpa Stalled (r:0 w:1)
	fn note_stalled() -> Weight {
		(6_440_000 as Weight)
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
}