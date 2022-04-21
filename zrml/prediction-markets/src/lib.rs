#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod benchmarks;
mod migrations;
pub mod mock;
mod tests;
pub mod weights;

pub use pallet::*;

#[frame_support::pallet]
mod pallet {
    use crate::weights::*;
    use alloc::{vec, vec::Vec};
    use core::{marker::PhantomData, ops::RangeInclusive};
    use frame_support::{
        dispatch::{DispatchResultWithPostInfo, Weight},
        ensure,
        pallet_prelude::{ConstU32, StorageValue, ValueQuery},
        traits::{
            Currency, EnsureOrigin, ExistenceRequirement, Get, IsType, NamedReservableCurrency,
            OnUnbalanced, StorageVersion,
        },
        transactional, BoundedVec, PalletId,
    };
    use frame_system::{ensure_signed, pallet_prelude::OriginFor};
    use orml_traits::MultiCurrency;
    use sp_runtime::{
        traits::{AccountIdConversion, Saturating, Zero},
        DispatchError, DispatchResult, SaturatedConversion,
    };
    use zeitgeist_primitives::{
        constants::{PmPalletId, MILLISECS_PER_BLOCK},
        traits::{DisputeApi, Swaps, ZeitgeistMultiReservableCurrency},
        types::{
            Asset, Market, MarketCreation, MarketDispute, MarketDisputeMechanism, MarketPeriod,
            MarketStatus, MarketType, MultiHash, OutcomeReport, Report, ScalarPosition,
            ScoringRule, SubsidyUntil,
        },
    };
    use zrml_liquidity_mining::LiquidityMiningPalletApi;
    use zrml_market_commons::MarketCommonsPalletApi;

    pub(crate) const RESERVE_ID: [u8; 8] = PmPalletId::get().0;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

    pub(crate) type BalanceOf<T> =
        <CurrencyOf<T> as Currency<<T as frame_system::Config>::AccountId>>::Balance;
    pub(crate) type CurrencyOf<T> =
        <<T as Config>::MarketCommons as MarketCommonsPalletApi>::Currency;
    pub(crate) type MarketIdOf<T> =
        <<T as Config>::MarketCommons as MarketCommonsPalletApi>::MarketId;
    type NegativeImbalanceOf<T> =
        <CurrencyOf<T> as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;
    pub(crate) type MomentOf<T> = <<T as Config>::MarketCommons as MarketCommonsPalletApi>::Moment;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(
            T::WeightInfo::buy_complete_set(T::MaxCategories::get().into())
        )]
        #[transactional]
        pub fn buy_complete_set(
            origin: OriginFor<T>,
            market_id: MarketIdOf<T>,
            #[pallet::compact] amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            Self::do_buy_complete_set(sender, market_id, amount)
        }

        #[pallet::weight(T::WeightInfo::create_categorical_market())]
        #[transactional]
        pub fn create_categorical_market(
            origin: OriginFor<T>,
            oracle: T::AccountId,
            period: MarketPeriod<T::BlockNumber, MomentOf<T>>,
            metadata: MultiHash,
            creation: MarketCreation,
            categories: u16,
            mdm: MarketDisputeMechanism<T::AccountId>,
            scoring_rule: ScoringRule,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            Self::ensure_market_is_active(&period)?;

            ensure!(categories >= T::MinCategories::get(), <Error<T>>::NotEnoughCategories);
            ensure!(categories <= T::MaxCategories::get(), <Error<T>>::TooManyCategories);

            if scoring_rule == ScoringRule::RikiddoSigmoidFeeMarketEma {
                Self::ensure_market_start_is_in_time(&period)?;
            }

            let MultiHash::Sha3_384(multihash) = metadata;
            ensure!(multihash[0] == 0x15 && multihash[1] == 0x30, <Error<T>>::InvalidMultihash);

            let status: MarketStatus = match creation {
                MarketCreation::Permissionless => {
                    let required_bond = T::ValidityBond::get() + T::OracleBond::get();
                    CurrencyOf::<T>::reserve_named(&RESERVE_ID, &sender, required_bond)?;

                    if scoring_rule == ScoringRule::CPMM {
                        MarketStatus::Active
                    } else {
                        MarketStatus::CollectingSubsidy
                    }
                }
                MarketCreation::Advised => {
                    let required_bond = T::AdvisoryBond::get() + T::OracleBond::get();
                    CurrencyOf::<T>::reserve_named(&RESERVE_ID, &sender, required_bond)?;
                    MarketStatus::Proposed
                }
            };

            let market = Market {
                creation,
                creator_fee: 0,
                creator: sender,
                market_type: MarketType::Categorical(categories),
                mdm,
                metadata: Vec::from(multihash),
                oracle,
                period,
                report: None,
                resolved_outcome: None,
                scoring_rule,
                status,
            };
            let market_id = T::MarketCommons::push_market(market.clone())?;
            let mut extra_weight = 0;

            if market.status == MarketStatus::CollectingSubsidy {
                extra_weight = Self::start_subsidy(&market, market_id)?;
            }

            Self::deposit_event(Event::MarketCreated(market_id, market));

            Ok(Some(T::WeightInfo::create_categorical_market().saturating_add(extra_weight)).into())
        }

        #[pallet::weight(T::WeightInfo::create_scalar_market())]
        #[transactional]
        pub fn create_scalar_market(
            origin: OriginFor<T>,
            oracle: T::AccountId,
            period: MarketPeriod<T::BlockNumber, MomentOf<T>>,
            metadata: MultiHash,
            creation: MarketCreation,
            outcome_range: RangeInclusive<u128>,
            mdm: MarketDisputeMechanism<T::AccountId>,
            scoring_rule: ScoringRule,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            Self::ensure_market_is_active(&period)?;

            ensure!(outcome_range.start() < outcome_range.end(), "Invalid range provided.");

            if scoring_rule == ScoringRule::RikiddoSigmoidFeeMarketEma {
                Self::ensure_market_start_is_in_time(&period)?;
            }

            let MultiHash::Sha3_384(multihash) = metadata;
            ensure!(multihash[0] == 0x15 && multihash[1] == 0x30, <Error<T>>::InvalidMultihash);

            let status: MarketStatus = match creation {
                MarketCreation::Permissionless => {
                    let required_bond = T::ValidityBond::get() + T::OracleBond::get();
                    CurrencyOf::<T>::reserve_named(&RESERVE_ID, &sender, required_bond)?;

                    if scoring_rule == ScoringRule::CPMM {
                        MarketStatus::Active
                    } else {
                        MarketStatus::CollectingSubsidy
                    }
                }
                MarketCreation::Advised => {
                    let required_bond = T::AdvisoryBond::get() + T::OracleBond::get();
                    CurrencyOf::<T>::reserve_named(&RESERVE_ID, &sender, required_bond)?;
                    MarketStatus::Proposed
                }
            };

            let market = Market {
                creation,
                creator_fee: 0,
                creator: sender,
                market_type: MarketType::Scalar(outcome_range),
                mdm,
                metadata: Vec::from(multihash),
                oracle,
                period,
                report: None,
                resolved_outcome: None,
                status,
                scoring_rule,
            };
            let market_id = T::MarketCommons::push_market(market.clone())?;
            let mut extra_weight = 0;

            if market.status == MarketStatus::CollectingSubsidy {
                extra_weight = Self::start_subsidy(&market, market_id)?;
            }

            Self::deposit_event(Event::MarketCreated(market_id, market));

            Ok(Some(T::WeightInfo::create_scalar_market().saturating_add(extra_weight)).into())
        }
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        #[pallet::constant]
        type AdvisoryBond: Get<BalanceOf<Self>>;

        type ApprovalOrigin: EnsureOrigin<Self::Origin>;

        type Authorized: zrml_authorized::AuthorizedPalletApi<
            AccountId = Self::AccountId,
            Balance = BalanceOf<Self>,
            BlockNumber = Self::BlockNumber,
            MarketId = MarketIdOf<Self>,
            Moment = MomentOf<Self>,
            Origin = Self::Origin,
        >;

        type CloseOrigin: EnsureOrigin<Self::Origin>;

        type Court: zrml_court::CourtPalletApi<
            AccountId = Self::AccountId,
            Balance = BalanceOf<Self>,
            BlockNumber = Self::BlockNumber,
            MarketId = MarketIdOf<Self>,
            Moment = MomentOf<Self>,
            Origin = Self::Origin,
        >;

        type DestroyOrigin: EnsureOrigin<Self::Origin>;

        #[pallet::constant]
        type DisputeBond: Get<BalanceOf<Self>>;

        #[pallet::constant]
        type DisputeFactor: Get<BalanceOf<Self>>;

        #[pallet::constant]
        type DisputePeriod: Get<Self::BlockNumber>;

        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        type LiquidityMining: LiquidityMiningPalletApi<
            AccountId = Self::AccountId,
            Balance = BalanceOf<Self>,
            BlockNumber = Self::BlockNumber,
            MarketId = MarketIdOf<Self>,
        >;

        type MarketCommons: MarketCommonsPalletApi<
            AccountId = Self::AccountId,
            BlockNumber = Self::BlockNumber,
        >;

        #[pallet::constant]
        type MaxCategories: Get<u16>;

        #[pallet::constant]
        type MaxSubsidyPeriod: Get<MomentOf<Self>>;

        #[pallet::constant]
        type MinCategories: Get<u16>;

        #[pallet::constant]
        type MinSubsidyPeriod: Get<MomentOf<Self>>;

        #[pallet::constant]
        type MaxDisputes: Get<u32>;

        type Shares: ZeitgeistMultiReservableCurrency<
            Self::AccountId,
            Balance = BalanceOf<Self>,
            CurrencyId = Asset<MarketIdOf<Self>>,
        >;

        #[pallet::constant]
        type PalletId: Get<PalletId>;

        #[pallet::constant]
        type OracleBond: Get<BalanceOf<Self>>;

        #[pallet::constant]
        type ReportingPeriod: Get<u32>;

        type ResolveOrigin: EnsureOrigin<Self::Origin>;

        type SimpleDisputes: DisputeApi<
            AccountId = Self::AccountId,
            Balance = BalanceOf<Self>,
            BlockNumber = Self::BlockNumber,
            MarketId = MarketIdOf<Self>,
            Moment = MomentOf<Self>,
            Origin = Self::Origin,
        >;

        type Slash: OnUnbalanced<NegativeImbalanceOf<Self>>;

        type Swaps: Swaps<Self::AccountId, Balance = BalanceOf<Self>, MarketId = MarketIdOf<Self>>;

        #[pallet::constant]
        type ValidityBond: Get<BalanceOf<Self>>;

        type WeightInfo: WeightInfoZeitgeist;
    }

    #[pallet::error]
    pub enum Error<T> {
        CannotDisputeSameOutcome,
        InsufficientFundsInMarketAccount,
        InsufficientShareBalance,
        InvalidMultihash,
        InvalidMarketType,
        InvalidScoringRule,
        NotEnoughBalance,
        OutcomeOutOfRange,
        MarketAlreadyReported,
        MarketIsNotActive,
        MarketIsNotClosed,
        MarketIsNotCollectingSubsidy,
        MarketIsNotProposed,
        MarketIsNotReported,
        MarketIsNotResolved,
        MarketNotReported,
        MarketStartTooSoon,
        MarketStartTooLate,
        MaxDisputesReached,
        NotEnoughAssets,
        NotEnoughCategories,
        NoWinningBalance,
        OutcomeMismatch,
        ReporterNotOracle,
        StorageOverflow,
        SwapPoolExists,
        TooManyCategories,
        InvalidMarketStatus,
        ZeroAmount,
    }

    #[pallet::event]
    #[pallet::generate_deposit(fn deposit_event)]
    pub enum Event<T>
    where
        T: Config,
    {
        BadOnInitialize,
        BoughtCompleteSet(MarketIdOf<T>, BalanceOf<T>, <T as frame_system::Config>::AccountId),
        MarketApproved(MarketIdOf<T>, MarketStatus),
        MarketCreated(MarketIdOf<T>, Market<T::AccountId, T::BlockNumber, MomentOf<T>>),
        MarketDestroyed(MarketIdOf<T>),
        MarketStartedWithSubsidy(MarketIdOf<T>, MarketStatus),
        MarketInsufficientSubsidy(MarketIdOf<T>, MarketStatus),
        MarketCancelled(MarketIdOf<T>),
        MarketDisputed(MarketIdOf<T>, MarketStatus, MarketDispute<T::AccountId, T::BlockNumber>),
        MarketRejected(MarketIdOf<T>),
        MarketReported(MarketIdOf<T>, MarketStatus, Report<T::AccountId, T::BlockNumber>),
        MarketResolved(MarketIdOf<T>, MarketStatus, OutcomeReport),
        SoldCompleteSet(MarketIdOf<T>, BalanceOf<T>, <T as frame_system::Config>::AccountId),
        TokensRedeemed(
            MarketIdOf<T>,
            Asset<MarketIdOf<T>>,
            BalanceOf<T>,
            BalanceOf<T>,
            <T as frame_system::Config>::AccountId,
        ),
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::storage]
    pub type MarketsCollectingSubsidy<T: Config> = StorageValue<
        _,
        BoundedVec<SubsidyUntil<T::BlockNumber, MomentOf<T>, MarketIdOf<T>>, ConstU32<1_048_576>>,
        ValueQuery,
    >;

    impl<T: Config> Pallet<T> {
        pub fn outcome_assets(
            market_id: MarketIdOf<T>,
            market: &Market<T::AccountId, T::BlockNumber, MomentOf<T>>,
        ) -> Vec<Asset<MarketIdOf<T>>> {
            match market.market_type {
                MarketType::Categorical(categories) => {
                    let mut assets = Vec::new();
                    for i in 0..categories {
                        assets.push(Asset::CategoricalOutcome(market_id, i));
                    }
                    assets
                }
                MarketType::Scalar(_) => {
                    vec![
                        Asset::ScalarOutcome(market_id, ScalarPosition::Long),
                        Asset::ScalarOutcome(market_id, ScalarPosition::Short),
                    ]
                }
            }
        }

        pub(crate) fn market_account(market_id: MarketIdOf<T>) -> T::AccountId {
            T::PalletId::get().into_sub_account(market_id.saturated_into::<u128>())
        }

        pub(crate) fn do_buy_complete_set(
            who: T::AccountId,
            market_id: MarketIdOf<T>,
            amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            ensure!(amount != BalanceOf::<T>::zero(), Error::<T>::ZeroAmount);
            ensure!(CurrencyOf::<T>::free_balance(&who) >= amount, Error::<T>::NotEnoughBalance);

            let market = T::MarketCommons::market(&market_id)?;
            ensure!(market.scoring_rule == ScoringRule::CPMM, Error::<T>::InvalidScoringRule);
            Self::ensure_market_is_active(&market.period)?;
            ensure!(market.status == MarketStatus::Active, Error::<T>::MarketIsNotActive);

            let market_account = Self::market_account(market_id);
            CurrencyOf::<T>::transfer(
                &who,
                &market_account,
                amount,
                ExistenceRequirement::KeepAlive,
            )?;

            let assets = Self::outcome_assets(market_id, &market);
            for asset in assets.iter() {
                T::Shares::deposit(*asset, &who, amount)?;
            }

            Self::deposit_event(Event::BoughtCompleteSet(market_id, amount, who));

            let assets_len: u32 = assets.len().saturated_into();
            let max_cats: u32 = T::MaxCategories::get().into();
            Self::calculate_actual_weight(&T::WeightInfo::buy_complete_set, assets_len, max_cats)
        }

        fn calculate_actual_weight<F>(
            func: F,
            weight_parameter: u32,
            max_weight_parameter: u32,
        ) -> DispatchResultWithPostInfo
        where
            F: Fn(u32) -> Weight,
        {
            if weight_parameter == max_weight_parameter {
                Ok(None.into())
            } else {
                Ok(Some(func(weight_parameter)).into())
            }
        }

        fn ensure_market_is_active(
            period: &MarketPeriod<T::BlockNumber, MomentOf<T>>,
        ) -> DispatchResult {
            ensure!(
                match period {
                    MarketPeriod::Block(range) => {
                        <frame_system::Pallet<T>>::block_number() < range.end
                    }
                    MarketPeriod::Timestamp(range) => {
                        T::MarketCommons::now() < range.end
                    }
                },
                Error::<T>::MarketIsNotActive
            );
            Ok(())
        }

        fn ensure_market_start_is_in_time(
            period: &MarketPeriod<T::BlockNumber, MomentOf<T>>,
        ) -> DispatchResult {
            let interval = match period {
                MarketPeriod::Block(range) => {
                    let interval_blocks: u128 = range
                        .start
                        .saturating_sub(<frame_system::Pallet<T>>::block_number())
                        .saturated_into();
                    interval_blocks.saturating_mul(MILLISECS_PER_BLOCK.into())
                }
                MarketPeriod::Timestamp(range) => {
                    range.start.saturating_sub(T::MarketCommons::now()).saturated_into()
                }
            };

            ensure!(
                <MomentOf<T>>::saturated_from(interval) >= T::MinSubsidyPeriod::get(),
                <Error<T>>::MarketStartTooSoon
            );
            ensure!(
                <MomentOf<T>>::saturated_from(interval) <= T::MaxSubsidyPeriod::get(),
                <Error<T>>::MarketStartTooLate
            );
            Ok(())
        }

        pub(crate) fn start_subsidy(
            market: &Market<T::AccountId, T::BlockNumber, MomentOf<T>>,
            market_id: MarketIdOf<T>,
        ) -> Result<Weight, DispatchError> {
            ensure!(T::MarketCommons::market_pool(&market_id).is_err(), Error::<T>::SwapPoolExists);
            ensure!(
                market.status == MarketStatus::CollectingSubsidy,
                Error::<T>::MarketIsNotCollectingSubsidy
            );

            let mut assets = Self::outcome_assets(market_id, market);
            let base_asset = Asset::Ztg;
            assets.push(base_asset);
            let total_assets = assets.len();

            let pool_id = T::Swaps::create_pool(
                market.creator.clone(),
                assets,
                base_asset,
                market_id,
                market.scoring_rule,
                None,
                None,
            )?;

            T::MarketCommons::insert_market_pool(market_id, pool_id);
            <MarketsCollectingSubsidy<T>>::try_mutate(|markets| {
                markets
                    .try_push(SubsidyUntil { market_id, period: market.period.clone() })
                    .map_err(|_| <Error<T>>::StorageOverflow)
            })?;

            Ok(T::WeightInfo::start_subsidy(total_assets.saturated_into()))
        }
    }

    pub fn default_dispute_bond<T>(n: usize) -> BalanceOf<T>
    where
        T: Config,
    {
        T::DisputeBond::get().saturating_add(
            T::DisputeFactor::get().saturating_mul(n.saturated_into::<u32>().into()),
        )
    }
}
