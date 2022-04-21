extern crate alloc;

pub mod weights;

pub use pallet::*;

#[frame_support::pallet]
mod pallet {
    use crate::weights::*;
    use core::marker::PhantomData;
    use frame_support::{dispatch::DispatchResultWithPostInfo, traits::IsType, transactional};
    use frame_system::pallet_prelude::OriginFor;
    use zeitgeist_primitives::types::MultiHash;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(T::WeightInfo::create_categorical_market())]
        #[transactional]
        pub fn create_categorical_market(
            _: OriginFor<T>,
            metadata: MultiHash,
        ) -> DispatchResultWithPostInfo {
            let MultiHash::Sha3_384(_multihash) = metadata;
            Ok(Some(T::WeightInfo::create_categorical_market()).into())
        }
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        type WeightInfo: WeightInfoZeitgeist;
    }

    #[pallet::event]
    pub enum Event<T>
    where
        T: Config, {}

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);
}
