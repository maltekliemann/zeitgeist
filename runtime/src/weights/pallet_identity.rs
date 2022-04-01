//! Autogenerated weights for pallet_identity
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-03-23, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:// ./target/release/zeitgeist// benchmark// --chain=dev// --steps=50// --repeat=20// --pallet=pallet_identity// --extrinsic=*// --execution=wasm// --wasm-execution=compiled// --heap-pages=4096// --template=./misc/frame_weight_template.hbs// --output=./runtime/src/weights/
#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions for pallet_identity (automatically generated)
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_identity::weights::WeightInfo for WeightInfo<T> {
		// Storage: Identity Registrars (r:1 w:1)
	fn add_registrar(_r: u32, ) -> Weight {
		(30_715_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
		// Storage: Identity IdentityOf (r:1 w:1)
	fn set_identity(r: u32, x: u32, ) -> Weight {
		(50_628_000 as Weight)		
		// Standard Error: 291_000

			.saturating_add((1_775_000 as Weight).saturating_mul(r as Weight))		
		// Standard Error: 19_000

			.saturating_add((986_000 as Weight).saturating_mul(x as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
		// Storage: Identity IdentityOf (r:1 w:0)
		// Storage: Identity SubsOf (r:1 w:1)
		// Storage: Identity SuperOf (r:1 w:1)
	fn set_subs_new(s: u32, ) -> Weight {
		(71_720_000 as Weight)		
		// Standard Error: 87_000

			.saturating_add((8_398_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().reads((1 as Weight).saturating_mul(s as Weight)))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(s as Weight)))
	}
		// Storage: Identity IdentityOf (r:1 w:0)
		// Storage: Identity SubsOf (r:1 w:1)
		// Storage: Identity SuperOf (r:0 w:1)
	fn set_subs_old(p: u32, ) -> Weight {
		(64_149_000 as Weight)		
		// Standard Error: 26_000

			.saturating_add((2_457_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(p as Weight)))
	}
		// Storage: Identity SubsOf (r:1 w:1)
		// Storage: Identity IdentityOf (r:1 w:1)
		// Storage: Identity SuperOf (r:0 w:64)
	fn clear_identity(r: u32, s: u32, x: u32, ) -> Weight {
		(48_491_000 as Weight)		
		// Standard Error: 517_000

			.saturating_add((3_536_000 as Weight).saturating_mul(r as Weight))		
		// Standard Error: 29_000

			.saturating_add((2_469_000 as Weight).saturating_mul(s as Weight))		
		// Standard Error: 29_000

			.saturating_add((485_000 as Weight).saturating_mul(x as Weight))
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(s as Weight)))
	}
		// Storage: Identity Registrars (r:1 w:0)
		// Storage: Identity IdentityOf (r:1 w:1)
	fn request_judgement(r: u32, x: u32, ) -> Weight {
		(58_494_000 as Weight)		
		// Standard Error: 299_000

			.saturating_add((1_667_000 as Weight).saturating_mul(r as Weight))		
		// Standard Error: 20_000

			.saturating_add((1_018_000 as Weight).saturating_mul(x as Weight))
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
		// Storage: Identity IdentityOf (r:1 w:1)
	fn cancel_request(r: u32, x: u32, ) -> Weight {
		(56_688_000 as Weight)		
		// Standard Error: 322_000

			.saturating_add((1_301_000 as Weight).saturating_mul(r as Weight))		
		// Standard Error: 21_000

			.saturating_add((1_168_000 as Weight).saturating_mul(x as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
		// Storage: Identity Registrars (r:1 w:1)
	fn set_fee(r: u32, ) -> Weight {
		(12_393_000 as Weight)		
		// Standard Error: 71_000

			.saturating_add((387_000 as Weight).saturating_mul(r as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
		// Storage: Identity Registrars (r:1 w:1)
	fn set_account_id(r: u32, ) -> Weight {
		(13_178_000 as Weight)		
		// Standard Error: 62_000

			.saturating_add((189_000 as Weight).saturating_mul(r as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
		// Storage: Identity Registrars (r:1 w:1)
	fn set_fields(r: u32, ) -> Weight {
		(11_467_000 as Weight)		
		// Standard Error: 92_000

			.saturating_add((252_000 as Weight).saturating_mul(r as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
		// Storage: Identity Registrars (r:1 w:0)
		// Storage: Identity IdentityOf (r:1 w:1)
	fn provide_judgement(_r: u32, x: u32, ) -> Weight {
		(62_815_000 as Weight)		
		// Standard Error: 18_000

			.saturating_add((1_027_000 as Weight).saturating_mul(x as Weight))
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
		// Storage: Identity SubsOf (r:1 w:1)
		// Storage: Identity IdentityOf (r:1 w:1)
		// Storage: System Account (r:2 w:2)
		// Storage: Identity SuperOf (r:0 w:64)
	fn kill_identity(r: u32, s: u32, _x: u32, ) -> Weight {
		(57_458_000 as Weight)		
		// Standard Error: 473_000

			.saturating_add((5_746_000 as Weight).saturating_mul(r as Weight))		
		// Standard Error: 27_000

			.saturating_add((2_536_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(s as Weight)))
	}
		// Storage: Identity IdentityOf (r:1 w:0)
		// Storage: Identity SuperOf (r:1 w:1)
		// Storage: Identity SubsOf (r:1 w:1)
	fn add_sub(s: u32, ) -> Weight {
		(79_034_000 as Weight)		
		// Standard Error: 14_000

			.saturating_add((225_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
		// Storage: Identity IdentityOf (r:1 w:0)
		// Storage: Identity SuperOf (r:1 w:1)
	fn rename_sub(s: u32, ) -> Weight {
		(23_525_000 as Weight)		
		// Standard Error: 4_000

			.saturating_add((83_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
		// Storage: Identity IdentityOf (r:1 w:0)
		// Storage: Identity SuperOf (r:1 w:1)
		// Storage: Identity SubsOf (r:1 w:1)
	fn remove_sub(s: u32, ) -> Weight {
		(87_842_000 as Weight)		
		// Standard Error: 14_000

			.saturating_add((91_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
		// Storage: Identity SuperOf (r:1 w:1)
		// Storage: Identity SubsOf (r:1 w:1)
	fn quit_sub(s: u32, ) -> Weight {
		(55_002_000 as Weight)		
		// Standard Error: 11_000

			.saturating_add((268_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
}
