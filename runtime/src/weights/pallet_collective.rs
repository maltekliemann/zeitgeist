//! Autogenerated weights for pallet_collective
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-01-14, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// ./target/release/zeitgeist
// benchmark
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_collective
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

/// Weight functions for pallet_collective (automatically generated)
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_collective::weights::WeightInfo for WeightInfo<T> {

		// Storage: AdvisoryCommitteeCollective Members (r:1 w:1)

		// Storage: AdvisoryCommitteeCollective Proposals (r:1 w:0)

		// Storage: AdvisoryCommitteeCollective Voting (r:64 w:64)

		// Storage: AdvisoryCommitteeCollective Prime (r:0 w:1)

	fn set_members(m: u32, n: u32, p: u32, ) -> Weight {
		(0 as Weight)
		
		// Standard Error: 20_000

			.saturating_add((17_901_000 as Weight).saturating_mul(m as Weight))
		
		// Standard Error: 20_000

			.saturating_add((335_000 as Weight).saturating_mul(n as Weight))
		
		// Standard Error: 35_000

			.saturating_add((39_439_000 as Weight).saturating_mul(p as Weight))

			.saturating_add(T::DbWeight::get().reads(2 as Weight))

			.saturating_add(T::DbWeight::get().reads((1 as Weight).saturating_mul(p as Weight)))

			.saturating_add(T::DbWeight::get().writes(2 as Weight))

			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(p as Weight)))

	}

		// Storage: AdvisoryCommitteeCollective Members (r:1 w:0)

	fn execute(b: u32, m: u32, ) -> Weight {
		(49_490_000 as Weight)
		
		// Standard Error: 0

			.saturating_add((4_000 as Weight).saturating_mul(b as Weight))
		
		// Standard Error: 0

			.saturating_add((175_000 as Weight).saturating_mul(m as Weight))

			.saturating_add(T::DbWeight::get().reads(1 as Weight))

	}

		// Storage: AdvisoryCommitteeCollective Members (r:1 w:0)

		// Storage: AdvisoryCommitteeCollective ProposalOf (r:1 w:0)

	fn propose_execute(b: u32, m: u32, ) -> Weight {
		(62_713_000 as Weight)
		
		// Standard Error: 0

			.saturating_add((4_000 as Weight).saturating_mul(b as Weight))
		
		// Standard Error: 2_000

			.saturating_add((324_000 as Weight).saturating_mul(m as Weight))

			.saturating_add(T::DbWeight::get().reads(2 as Weight))

	}

		// Storage: AdvisoryCommitteeCollective Members (r:1 w:0)

		// Storage: AdvisoryCommitteeCollective ProposalOf (r:1 w:1)

		// Storage: AdvisoryCommitteeCollective Proposals (r:1 w:1)

		// Storage: AdvisoryCommitteeCollective ProposalCount (r:1 w:1)

		// Storage: AdvisoryCommitteeCollective Voting (r:0 w:1)

	fn propose_proposed(b: u32, m: u32, p: u32, ) -> Weight {
		(77_941_000 as Weight)
		
		// Standard Error: 0

			.saturating_add((13_000 as Weight).saturating_mul(b as Weight))
		
		// Standard Error: 5_000

			.saturating_add((194_000 as Weight).saturating_mul(m as Weight))
		
		// Standard Error: 8_000

			.saturating_add((862_000 as Weight).saturating_mul(p as Weight))

			.saturating_add(T::DbWeight::get().reads(4 as Weight))

			.saturating_add(T::DbWeight::get().writes(4 as Weight))

	}

		// Storage: AdvisoryCommitteeCollective Members (r:1 w:0)

		// Storage: AdvisoryCommitteeCollective Voting (r:1 w:1)

	fn vote(m: u32, ) -> Weight {
		(74_572_000 as Weight)
		
		// Standard Error: 4_000

			.saturating_add((371_000 as Weight).saturating_mul(m as Weight))

			.saturating_add(T::DbWeight::get().reads(2 as Weight))

			.saturating_add(T::DbWeight::get().writes(1 as Weight))

	}

		// Storage: AdvisoryCommitteeCollective Voting (r:1 w:1)

		// Storage: AdvisoryCommitteeCollective Members (r:1 w:0)

		// Storage: AdvisoryCommitteeCollective Proposals (r:1 w:1)

		// Storage: AdvisoryCommitteeCollective ProposalOf (r:0 w:1)

	fn close_early_disapproved(m: u32, p: u32, ) -> Weight {
		(86_182_000 as Weight)
		
		// Standard Error: 4_000

			.saturating_add((330_000 as Weight).saturating_mul(m as Weight))
		
		// Standard Error: 7_000

			.saturating_add((796_000 as Weight).saturating_mul(p as Weight))

			.saturating_add(T::DbWeight::get().reads(3 as Weight))

			.saturating_add(T::DbWeight::get().writes(3 as Weight))

	}

		// Storage: AdvisoryCommitteeCollective Voting (r:1 w:1)

		// Storage: AdvisoryCommitteeCollective Members (r:1 w:0)

		// Storage: AdvisoryCommitteeCollective ProposalOf (r:1 w:1)

		// Storage: AdvisoryCommitteeCollective Proposals (r:1 w:1)

	fn close_early_approved(b: u32, m: u32, p: u32, ) -> Weight {
		(109_707_000 as Weight)
		
		// Standard Error: 0

			.saturating_add((9_000 as Weight).saturating_mul(b as Weight))
		
		// Standard Error: 5_000

			.saturating_add((362_000 as Weight).saturating_mul(m as Weight))
		
		// Standard Error: 9_000

			.saturating_add((890_000 as Weight).saturating_mul(p as Weight))

			.saturating_add(T::DbWeight::get().reads(4 as Weight))

			.saturating_add(T::DbWeight::get().writes(3 as Weight))

	}

		// Storage: AdvisoryCommitteeCollective Voting (r:1 w:1)

		// Storage: AdvisoryCommitteeCollective Members (r:1 w:0)

		// Storage: AdvisoryCommitteeCollective Prime (r:1 w:0)

		// Storage: AdvisoryCommitteeCollective Proposals (r:1 w:1)

		// Storage: AdvisoryCommitteeCollective ProposalOf (r:0 w:1)

	fn close_disapproved(m: u32, p: u32, ) -> Weight {
		(99_746_000 as Weight)
		
		// Standard Error: 5_000

			.saturating_add((324_000 as Weight).saturating_mul(m as Weight))
		
		// Standard Error: 8_000

			.saturating_add((756_000 as Weight).saturating_mul(p as Weight))

			.saturating_add(T::DbWeight::get().reads(4 as Weight))

			.saturating_add(T::DbWeight::get().writes(3 as Weight))

	}

		// Storage: AdvisoryCommitteeCollective Voting (r:1 w:1)

		// Storage: AdvisoryCommitteeCollective Members (r:1 w:0)

		// Storage: AdvisoryCommitteeCollective Prime (r:1 w:0)

		// Storage: AdvisoryCommitteeCollective ProposalOf (r:1 w:1)

		// Storage: AdvisoryCommitteeCollective Proposals (r:1 w:1)

	fn close_approved(b: u32, m: u32, p: u32, ) -> Weight {
		(122_098_000 as Weight)
		
		// Standard Error: 0

			.saturating_add((6_000 as Weight).saturating_mul(b as Weight))
		
		// Standard Error: 5_000

			.saturating_add((359_000 as Weight).saturating_mul(m as Weight))
		
		// Standard Error: 9_000

			.saturating_add((918_000 as Weight).saturating_mul(p as Weight))

			.saturating_add(T::DbWeight::get().reads(5 as Weight))

			.saturating_add(T::DbWeight::get().writes(3 as Weight))

	}

		// Storage: AdvisoryCommitteeCollective Proposals (r:1 w:1)

		// Storage: AdvisoryCommitteeCollective Voting (r:0 w:1)

		// Storage: AdvisoryCommitteeCollective ProposalOf (r:0 w:1)

	fn disapprove_proposal(p: u32, ) -> Weight {
		(55_659_000 as Weight)
		
		// Standard Error: 5_000

			.saturating_add((732_000 as Weight).saturating_mul(p as Weight))

			.saturating_add(T::DbWeight::get().reads(1 as Weight))

			.saturating_add(T::DbWeight::get().writes(3 as Weight))

	}
}
