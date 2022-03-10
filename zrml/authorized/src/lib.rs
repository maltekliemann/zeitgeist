//! # Authorized

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod authorized_pallet_api;
mod benchmarks;
mod mock;
mod tests;
pub mod weights;

pub use authorized_pallet_api::AuthorizedPalletApi;
pub use pallet::*;

#[frame_support::pallet]
mod pallet {
    use crate::{weights::WeightInfoZeitgeist, AuthorizedPalletApi};
    use core::marker::PhantomData;
    use frame_support::{
        dispatch::DispatchResult,
        pallet_prelude::StorageDoubleMap,
        traits::{Currency, Get, Hooks, IsType, StorageVersion},
        Blake2_128Concat, PalletId,
    };
    use frame_system::{ensure_signed, pallet_prelude::OriginFor};
    use sp_runtime::DispatchError;
    use zeitgeist_primitives::{
        traits::DisputeApi,
        types::{Market, MarketDispute, MarketDisputeMechanism, OutcomeReport},
    };
    use zrml_market_commons::MarketCommonsPalletApi;

    /// The current storage version.
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

    pub(crate) type BalanceOf<T> =
        <CurrencyOf<T> as Currency<<T as frame_system::Config>::AccountId>>::Balance;
    pub(crate) type CurrencyOf<T> =
        <<T as Config>::MarketCommons as MarketCommonsPalletApi>::Currency;
    pub(crate) type MarketIdOf<T> =
        <<T as Config>::MarketCommons as MarketCommonsPalletApi>::MarketId;
    pub(crate) type MomentOf<T> = <<T as Config>::MarketCommons as MarketCommonsPalletApi>::Moment;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Overwrites already provided outcomes for the same market and account.
        #[frame_support::transactional]
        #[pallet::weight(T::WeightInfo::authorize_market_outcome())]
        pub fn authorize_market_outcome(
            origin: OriginFor<T>,
            outcome: OutcomeReport,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let markets = T::MarketCommons::markets();
            let market_id = if let Some(rslt) = markets.iter().find(|el| {
                if let MarketDisputeMechanism::Authorized(ref account_id) = el.1.mdm {
                    account_id == &who
                } else {
                    false
                }
            }) {
                rslt.0
            } else {
                return Err(Error::<T>::AccountIsNotLinkedToAnyAuthorizedMarket.into());
            };

            Outcomes::<T>::insert(market_id, who, outcome);

            // An already stored market was probably modified with a different
            // authorized account.
            if Outcomes::<T>::iter_prefix(market_id).count() > 1 {
                return Err(Error::<T>::MarketsCanNotHaveMoreThanOneAuthorizedAccount.into());
            }

            Ok(())
        }
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Event
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Market commons
        type MarketCommons: MarketCommonsPalletApi<
            AccountId = Self::AccountId,
            BlockNumber = Self::BlockNumber,
        >;

        /// Identifier of this pallet
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Weights generated by benchmarks
        type WeightInfo: WeightInfoZeitgeist;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// An account trying to register an outcome is not tied to any authorized market.
        AccountIsNotLinkedToAnyAuthorizedMarket,
        /// On dispute or resolution, someone tried to pass a non-authorized market type
        MarketDoesNotHaveAuthorizedMechanism,
        /// It is not possible to have more than one stored outcome for the same market.
        MarketsCanNotHaveMoreThanOneAuthorizedAccount,
        /// On resolution, someone tried to pass a unknown account id or market id.
        UnknownOutcome,
    }

    #[pallet::event]
    pub enum Event<T>
    where
        T: Config, {}

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(PhantomData<T>);

    impl<T> Pallet<T> where T: Config {}

    impl<T> DisputeApi for Pallet<T>
    where
        T: Config,
    {
        type AccountId = T::AccountId;
        type Balance = BalanceOf<T>;
        type BlockNumber = T::BlockNumber;
        type MarketId = MarketIdOf<T>;
        type Moment = MomentOf<T>;
        type Origin = T::Origin;

        fn on_dispute(
            _: &[MarketDispute<Self::AccountId, Self::BlockNumber>],
            _: &Self::MarketId,
            market: &Market<Self::AccountId, Self::BlockNumber, Self::Moment>,
        ) -> DispatchResult {
            if let MarketDisputeMechanism::Authorized(_) = market.mdm {
                Ok(())
            } else {
                Err(Error::<T>::MarketDoesNotHaveAuthorizedMechanism.into())
            }
        }

        fn on_resolution(
            _: &[MarketDispute<Self::AccountId, Self::BlockNumber>],
            market_id: &Self::MarketId,
            market: &Market<Self::AccountId, Self::BlockNumber, MomentOf<T>>,
        ) -> Result<OutcomeReport, DispatchError> {
            let market_ai = if let MarketDisputeMechanism::Authorized(ref el) = market.mdm {
                el
            } else {
                return Err(Error::<T>::MarketDoesNotHaveAuthorizedMechanism.into());
            };

            let outcome = if let Some(el) = Outcomes::<T>::get(market_id, market_ai) {
                el
            } else {
                return Err(Error::<T>::UnknownOutcome.into());
            };

            Outcomes::<T>::remove(market_id, market_ai);

            Ok(outcome)
        }
    }

    impl<T> AuthorizedPalletApi for Pallet<T> where T: Config {}

    /// Reported outcomes of accounts that are linked through
    /// `MarketDisputeMechanism::Authorized(..)`.
    #[pallet::storage]
    pub type Outcomes<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        MarketIdOf<T>,
        Blake2_128Concat,
        T::AccountId,
        OutcomeReport,
    >;
}

#[cfg(any(feature = "runtime-benchmarks", test))]
pub(crate) fn market_mock<T>(
    ai: T::AccountId,
) -> zeitgeist_primitives::types::Market<T::AccountId, T::BlockNumber, MomentOf<T>>
where
    T: crate::Config,
{
    use zeitgeist_primitives::types::ScoringRule;

    zeitgeist_primitives::types::Market {
        creation: zeitgeist_primitives::types::MarketCreation::Permissionless,
        creator_fee: 0,
        creator: T::AccountId::default(),
        market_type: zeitgeist_primitives::types::MarketType::Scalar(0..=100),
        mdm: zeitgeist_primitives::types::MarketDisputeMechanism::Authorized(ai),
        metadata: Default::default(),
        oracle: T::AccountId::default(),
        period: zeitgeist_primitives::types::MarketPeriod::Block(Default::default()),
        report: None,
        resolved_outcome: None,
        scoring_rule: ScoringRule::CPMM,
        status: zeitgeist_primitives::types::MarketStatus::Closed,
    }
}
