//! Autogenerated weights for pallet_identity
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-04-15, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/zeitgeist
// benchmark
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_identity
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --template=./misc/frame_weight_template.hbs
// --output=./runtime/src/weights/

#![allow(unused_parens)]
#![allow(unused_imports)]

use core::marker::PhantomData;
use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};

/// Weight functions for pallet_identity (automatically generated)
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_identity::weights::WeightInfo for WeightInfo<T> {
    // Storage: Identity Registrars (r:1 w:1)
    fn add_registrar(r: u32) -> Weight {
        (33_895_000 as Weight)
            // Standard Error: 191_000
            .saturating_add((807_000 as Weight).saturating_mul(r as Weight))
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    // Storage: Identity IdentityOf (r:1 w:1)
    fn set_identity(_r: u32, x: u32) -> Weight {
        (89_974_000 as Weight)
            // Standard Error: 20_000
            .saturating_add((1_124_000 as Weight).saturating_mul(x as Weight))
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    // Storage: Identity IdentityOf (r:1 w:0)
    // Storage: Identity SubsOf (r:1 w:1)
    // Storage: Identity SuperOf (r:1 w:1)
    fn set_subs_new(s: u32) -> Weight {
        (85_761_000 as Weight)
            // Standard Error: 67_000
            .saturating_add((7_606_000 as Weight).saturating_mul(s as Weight))
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().reads((1 as Weight).saturating_mul(s as Weight)))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
            .saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(s as Weight)))
    }
    // Storage: Identity IdentityOf (r:1 w:0)
    // Storage: Identity SubsOf (r:1 w:1)
    // Storage: Identity SuperOf (r:0 w:1)
    fn set_subs_old(p: u32) -> Weight {
        (56_098_000 as Weight)
            // Standard Error: 14_000
            .saturating_add((2_300_000 as Weight).saturating_mul(p as Weight))
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
            .saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(p as Weight)))
    }
    // Storage: Identity SubsOf (r:1 w:1)
    // Storage: Identity IdentityOf (r:1 w:1)
    // Storage: Identity SuperOf (r:0 w:64)
    fn clear_identity(_r: u32, s: u32, x: u32) -> Weight {
        (86_342_000 as Weight)
            // Standard Error: 20_000
            .saturating_add((2_269_000 as Weight).saturating_mul(s as Weight))
            // Standard Error: 20_000
            .saturating_add((518_000 as Weight).saturating_mul(x as Weight))
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
            .saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(s as Weight)))
    }
    // Storage: Identity Registrars (r:1 w:0)
    // Storage: Identity IdentityOf (r:1 w:1)
    fn request_judgement(_r: u32, x: u32) -> Weight {
        (88_133_000 as Weight)
            // Standard Error: 16_000
            .saturating_add((1_093_000 as Weight).saturating_mul(x as Weight))
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    // Storage: Identity IdentityOf (r:1 w:1)
    fn cancel_request(_r: u32, x: u32) -> Weight {
        (75_264_000 as Weight)
            // Standard Error: 11_000
            .saturating_add((1_089_000 as Weight).saturating_mul(x as Weight))
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    // Storage: Identity Registrars (r:1 w:1)
    fn set_fee(r: u32) -> Weight {
        (12_809_000 as Weight)
            // Standard Error: 41_000
            .saturating_add((241_000 as Weight).saturating_mul(r as Weight))
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    // Storage: Identity Registrars (r:1 w:1)
    fn set_account_id(r: u32) -> Weight {
        (12_818_000 as Weight)
            // Standard Error: 36_000
            .saturating_add((210_000 as Weight).saturating_mul(r as Weight))
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    // Storage: Identity Registrars (r:1 w:1)
    fn set_fields(r: u32) -> Weight {
        (12_208_000 as Weight)
            // Standard Error: 30_000
            .saturating_add((397_000 as Weight).saturating_mul(r as Weight))
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    // Storage: Identity Registrars (r:1 w:0)
    // Storage: Identity IdentityOf (r:1 w:1)
    fn provide_judgement(_r: u32, x: u32) -> Weight {
        (61_475_000 as Weight)
            // Standard Error: 14_000
            .saturating_add((1_025_000 as Weight).saturating_mul(x as Weight))
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    // Storage: Identity SubsOf (r:1 w:1)
    // Storage: Identity IdentityOf (r:1 w:1)
    // Storage: System Account (r:2 w:2)
    // Storage: Identity SuperOf (r:0 w:64)
    fn kill_identity(_r: u32, s: u32, _x: u32) -> Weight {
        (97_797_000 as Weight)
            // Standard Error: 23_000
            .saturating_add((2_367_000 as Weight).saturating_mul(s as Weight))
            .saturating_add(T::DbWeight::get().reads(4 as Weight))
            .saturating_add(T::DbWeight::get().writes(4 as Weight))
            .saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(s as Weight)))
    }
    // Storage: Identity IdentityOf (r:1 w:0)
    // Storage: Identity SuperOf (r:1 w:1)
    // Storage: Identity SubsOf (r:1 w:1)
    fn add_sub(s: u32) -> Weight {
        (75_406_000 as Weight)
            // Standard Error: 12_000
            .saturating_add((322_000 as Weight).saturating_mul(s as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    // Storage: Identity IdentityOf (r:1 w:0)
    // Storage: Identity SuperOf (r:1 w:1)
    fn rename_sub(s: u32) -> Weight {
        (23_541_000 as Weight)
            // Standard Error: 2_000
            .saturating_add((65_000 as Weight).saturating_mul(s as Weight))
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    // Storage: Identity IdentityOf (r:1 w:0)
    // Storage: Identity SuperOf (r:1 w:1)
    // Storage: Identity SubsOf (r:1 w:1)
    fn remove_sub(s: u32) -> Weight {
        (77_234_000 as Weight)
            // Standard Error: 9_000
            .saturating_add((299_000 as Weight).saturating_mul(s as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    // Storage: Identity SuperOf (r:1 w:1)
    // Storage: Identity SubsOf (r:1 w:1)
    fn quit_sub(s: u32) -> Weight {
        (53_307_000 as Weight)
            // Standard Error: 7_000
            .saturating_add((192_000 as Weight).saturating_mul(s as Weight))
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
}
