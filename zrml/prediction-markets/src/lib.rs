
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
    use core::{cmp, marker::PhantomData, ops::RangeInclusive};
    use frame_support::{
        dispatch::{DispatchResultWithPostInfo, Weight},
        ensure, log,
        pallet_prelude::{ConstU32, StorageMap, StorageValue, ValueQuery},
        storage::{with_transaction, TransactionOutcome},
        traits::{
            Currency, EnsureOrigin, ExistenceRequirement, Get, Hooks, Imbalance, IsType,
            NamedReservableCurrency, OnUnbalanced, StorageVersion,
        },
        transactional, Blake2_128Concat, BoundedVec, PalletId, Twox64Concat,
    };
    use frame_system::{ensure_signed, pallet_prelude::OriginFor};
    use orml_traits::MultiCurrency;
    use sp_arithmetic::per_things::Perbill;
    use sp_runtime::{
        traits::{AccountIdConversion, CheckedDiv, Saturating, Zero},
        ArithmeticError, DispatchError, DispatchResult, SaturatedConversion,
    };
    use zeitgeist_primitives::{
        constants::{MinLiquidity, PmPalletId, MILLISECS_PER_BLOCK},
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
            T::WeightInfo::admin_destroy_reported_market(
                900,
                900,
                T::MaxCategories::get().into()
            ).max(T::WeightInfo::admin_destroy_disputed_market(
                900,
                900,
                T::MaxCategories::get().into()
            ))
        )]
        #[transactional]
        pub fn admin_destroy_market(
            origin: OriginFor<T>,
            market_id: MarketIdOf<T>,
        ) -> DispatchResultWithPostInfo {
            T::DestroyOrigin::ensure_origin(origin)?;

            let mut total_accounts = 0usize;
            let mut share_accounts = 0usize;
            let market = T::MarketCommons::market(&market_id)?;
            let market_status = market.status;
            let outcome_assets = Self::outcome_assets(market_id, &market);
            let outcome_assets_amount = outcome_assets.len();
            Self::clear_auto_resolve(&market_id)?;
            T::MarketCommons::remove_market(&market_id)?;
            Self::deposit_event(Event::MarketDestroyed(market_id));

            let mut outcome_assets_iter = outcome_assets.into_iter();

            let mut manage_outcome_asset = |asset: Asset<_>| -> usize {
                let (total_accounts, accounts) = T::Shares::accounts_by_currency_id(asset);
                share_accounts = share_accounts.saturating_add(accounts.len());
                T::Shares::destroy_all(asset, accounts.iter().cloned());
                total_accounts
            };

            if let Some(first_asset) = outcome_assets_iter.next() {
                total_accounts = manage_outcome_asset(first_asset);
            }
            for asset in outcome_assets_iter {
                let _ = manage_outcome_asset(asset);
            }

            if market_status == MarketStatus::Reported {
                Ok(Some(T::WeightInfo::admin_destroy_reported_market(
                    total_accounts.saturated_into(),
                    share_accounts.saturated_into(),
                    outcome_assets_amount.saturated_into(),
                ))
                .into())
            } else if market_status == MarketStatus::Disputed {
                Ok(Some(T::WeightInfo::admin_destroy_disputed_market(
                    total_accounts.saturated_into(),
                    share_accounts.saturated_into(),
                    outcome_assets_amount.saturated_into(),
                ))
                .into())
            } else {
                Ok(None.into())
            }
        }

        #[pallet::weight(T::WeightInfo::admin_move_market_to_closed())]
        #[transactional]
        pub fn admin_move_market_to_closed(
            origin: OriginFor<T>,
            market_id: MarketIdOf<T>,
        ) -> DispatchResult {
            T::CloseOrigin::ensure_origin(origin)?;
            T::MarketCommons::mutate_market(&market_id, |m| {
                m.period = match m.period {
                    MarketPeriod::Block(ref range) => {
                        let current_block = <frame_system::Pallet<T>>::block_number();
                        MarketPeriod::Block(range.start..current_block)
                    }
                    MarketPeriod::Timestamp(ref range) => {
                        let now = T::MarketCommons::now();
                        MarketPeriod::Timestamp(range.start..now)
                    }
                };
                Ok(())
            })?;
            Ok(())
        }

        #[pallet::weight(T::WeightInfo::admin_move_market_to_resolved_overhead()
            .saturating_add(T::WeightInfo::internal_resolve_categorical_reported(
                4_200,
                4_200,
                T::MaxCategories::get().into()
            ).saturating_sub(T::WeightInfo::internal_resolve_scalar_reported())
        ))]
        #[transactional]
        pub fn admin_move_market_to_resolved(
            origin: OriginFor<T>,
            market_id: MarketIdOf<T>,
        ) -> DispatchResultWithPostInfo {
            T::ResolveOrigin::ensure_origin(origin)?;

            let market = T::MarketCommons::market(&market_id)?;
            ensure!(
                market.status == MarketStatus::Reported || market.status == MarketStatus::Disputed,
                "not reported nor disputed"
            );
            Self::clear_auto_resolve(&market_id)?;
            let market = T::MarketCommons::market(&market_id)?;
            let weight = Self::on_resolution(&market_id, &market)?;
            Ok(Some(weight).into())
        }

        #[pallet::weight(T::WeightInfo::approve_market())]
        #[transactional]
        pub fn approve_market(
            origin: OriginFor<T>,
            market_id: MarketIdOf<T>,
        ) -> DispatchResultWithPostInfo {
            T::ApprovalOrigin::ensure_origin(origin)?;
            let mut extra_weight = 0;
            let mut status = MarketStatus::Active;

            T::MarketCommons::mutate_market(&market_id, |m| {
                ensure!(m.status == MarketStatus::Proposed, Error::<T>::MarketIsNotProposed);

                if m.scoring_rule == ScoringRule::CPMM {
                    m.status = MarketStatus::Active;
                } else {
                    m.status = MarketStatus::CollectingSubsidy;
                    status = MarketStatus::CollectingSubsidy;
                    extra_weight = Self::start_subsidy(m, market_id)?;
                }

                CurrencyOf::<T>::unreserve_named(&RESERVE_ID, &m.creator, T::AdvisoryBond::get());
                Ok(())
            })?;

            Self::deposit_event(Event::MarketApproved(market_id, status));
            Ok(Some(T::WeightInfo::approve_market().saturating_add(extra_weight)).into())
        }

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

        #[pallet::weight(T::WeightInfo::dispute(T::MaxDisputes::get()))]
        #[transactional]
        pub fn dispute(
            origin: OriginFor<T>,
            market_id: MarketIdOf<T>,
            outcome: OutcomeReport,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let disputes = Disputes::<T>::get(market_id);
            let curr_block_num = <frame_system::Pallet<T>>::block_number();
            let market = T::MarketCommons::market(&market_id)?;
            let num_disputes: u32 = disputes.len().saturated_into();
            Self::validate_dispute(&disputes, &market, num_disputes, &outcome)?;
            CurrencyOf::<T>::reserve_named(
                &RESERVE_ID,
                &who,
                default_dispute_bond::<T>(disputes.len()),
            )?;
            match market.mdm {
                MarketDisputeMechanism::Authorized(_) => {
                    T::Authorized::on_dispute(&disputes, &market_id, &market)?
                }
                MarketDisputeMechanism::Court => {
                    T::Court::on_dispute(&disputes, &market_id, &market)?
                }
                MarketDisputeMechanism::SimpleDisputes => {
                    T::SimpleDisputes::on_dispute(&disputes, &market_id, &market)?
                }
            }
            Self::remove_last_dispute_from_market_ids_per_dispute_block(&disputes, &market_id)?;
            Self::set_market_as_disputed(&market, &market_id)?;
            let market_dispute = MarketDispute { at: curr_block_num, by: who, outcome };
            <Disputes<T>>::try_mutate(market_id, |disputes| {
                disputes.try_push(market_dispute.clone()).map_err(|_| <Error<T>>::StorageOverflow)
            })?;
            <MarketIdsPerDisputeBlock<T>>::try_mutate(curr_block_num, |ids| {
                ids.try_push(market_id).map_err(|_| <Error<T>>::StorageOverflow)
            })?;
            Self::deposit_event(Event::MarketDisputed(
                market_id,
                MarketStatus::Disputed,
                market_dispute,
            ));
            Self::calculate_actual_weight(
                &T::WeightInfo::dispute,
                num_disputes,
                T::MaxDisputes::get(),
            )
        }

        #[pallet::weight(T::WeightInfo::cancel_pending_market())]
        #[transactional]
        pub fn cancel_pending_market(
            origin: OriginFor<T>,
            market_id: MarketIdOf<T>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let market = T::MarketCommons::market(&market_id)?;

            let creator = market.creator;
            let status = market.status;
            ensure!(creator == sender, "Canceller must be market creator.");
            ensure!(status == MarketStatus::Proposed, "Market must be pending approval.");
            CurrencyOf::<T>::unreserve_named(&RESERVE_ID, &creator, T::AdvisoryBond::get());
            T::MarketCommons::remove_market(&market_id)?;
            Self::deposit_event(Event::MarketCancelled(market_id));
            Self::deposit_event(Event::MarketDestroyed(market_id));
            Ok(())
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

        #[pallet::weight(
            T::WeightInfo::create_scalar_market().max(T::WeightInfo::create_categorical_market())
            .saturating_add(T::WeightInfo::buy_complete_set(T::MaxCategories::get().min(amount_outcome_assets.len().saturated_into()).into()))
            .saturating_add(T::WeightInfo::deploy_swap_pool_for_market(T::MaxCategories::get().min(weights.len().saturated_into()).into()))
            .saturating_add(5_000_000_000.saturating_mul(T::MaxCategories::get().min(amount_outcome_assets.len().saturated_into()).into()))
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
        )]
        #[transactional]
        pub fn create_cpmm_market_and_deploy_assets(
            origin: OriginFor<T>,
            oracle: T::AccountId,
            period: MarketPeriod<T::BlockNumber, MomentOf<T>>,
            metadata: MultiHash,
            assets: MarketType,
            mdm: MarketDisputeMechanism<T::AccountId>,
            amount_base_asset: BalanceOf<T>,
            amount_outcome_assets: Vec<BalanceOf<T>>,
            weights: Vec<u128>,
        ) -> DispatchResultWithPostInfo {
            let _ = ensure_signed(origin.clone())?;

            if let MarketType::Categorical(num_cat) = assets {
                ensure!(
                    amount_outcome_assets.len().saturated_into::<u16>() == num_cat,
                    Error::<T>::NotEnoughAssets
                );
            } else if let MarketType::Scalar(_) = assets {
                ensure!(amount_outcome_assets.len() == 2, Error::<T>::NotEnoughAssets);
            }

            let weight_market_creation = match assets.clone() {
                MarketType::Categorical(category_count) => Self::create_categorical_market(
                    origin.clone(),
                    oracle,
                    period,
                    metadata,
                    MarketCreation::Permissionless,
                    category_count,
                    mdm,
                    ScoringRule::CPMM,
                )?
                .actual_weight
                .unwrap_or_else(T::WeightInfo::create_categorical_market),
                MarketType::Scalar(range) => Self::create_scalar_market(
                    origin.clone(),
                    oracle,
                    period,
                    metadata,
                    MarketCreation::Permissionless,
                    range,
                    mdm,
                    ScoringRule::CPMM,
                )?
                .actual_weight
                .unwrap_or_else(T::WeightInfo::create_scalar_market),
            };

            let market_id = T::MarketCommons::latest_market_id()?;
            let deploy_and_populate_weight = Self::deploy_swap_pool_and_additional_liquidity(
                origin,
                market_id,
                amount_base_asset,
                amount_outcome_assets.clone(),
                weights.clone(),
            )?
            .actual_weight
            .unwrap_or_else(|| {
                T::WeightInfo::buy_complete_set(
                    T::MaxCategories::get()
                        .min(amount_outcome_assets.len().saturated_into())
                        .into(),
                )
                .saturating_add(T::WeightInfo::deploy_swap_pool_for_market(
                    T::MaxCategories::get().min(weights.len().saturated_into()).into(),
                ))
                .saturating_add(
                    5_000_000_000.saturating_mul(
                        T::MaxCategories::get()
                            .min(amount_outcome_assets.len().saturated_into())
                            .into(),
                    ),
                )
                .saturating_add(T::DbWeight::get().reads(2 as Weight))
            });

            Ok(Some(weight_market_creation.saturating_add(deploy_and_populate_weight)).into())
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

        #[pallet::weight(
            T::WeightInfo::buy_complete_set(T::MaxCategories::get().min(amount_outcome_assets.len().saturated_into()).into())
            .saturating_add(T::WeightInfo::deploy_swap_pool_for_market(T::MaxCategories::get().min(weights.len().saturated_into()).into()))
            .saturating_add(5_000_000_000.saturating_mul(T::MaxCategories::get().min(amount_outcome_assets.len().saturated_into()).into()))
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
        )]
        #[transactional]
        pub fn deploy_swap_pool_and_additional_liquidity(
            origin: OriginFor<T>,
            market_id: MarketIdOf<T>,
            amount_base_asset: BalanceOf<T>,
            amount_outcome_assets: Vec<BalanceOf<T>>,
            weights: Vec<u128>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin.clone())?;
            let assets = T::MarketCommons::market(&market_id)?.market_type;
            let zero_balance = <BalanceOf<T>>::zero();
            let max_assets = amount_outcome_assets
                .iter()
                .fold(zero_balance, |prev, cur| if prev > *cur { prev } else { *cur });
            let weight_bcs = Self::buy_complete_set(origin.clone(), market_id, max_assets)?
                .actual_weight
                .unwrap_or_else(|| T::WeightInfo::buy_complete_set(T::MaxCategories::get().into()));
            let weight_len = weights.len().saturated_into();

            let _ = Self::deploy_swap_pool_for_market(origin, market_id, weights)?;
            let pool_id = T::MarketCommons::market_pool(&market_id)?;
            let mut weight_pool_joins_and_sells = 0;
            let mut add_liqudity =
                |amount: BalanceOf<T>, asset: Asset<MarketIdOf<T>>| -> DispatchResult {
                    let local_weight = T::Swaps::pool_join_with_exact_asset_amount(
                        who.clone(),
                        pool_id,
                        asset,
                        amount,
                        zero_balance,
                    )?;
                    weight_pool_joins_and_sells =
                        weight_pool_joins_and_sells.saturating_add(local_weight);
                    Ok(())
                };

            for (idx, asset_amount) in amount_outcome_assets.iter().enumerate() {
                if *asset_amount == zero_balance {
                    continue;
                };

                let remaining_amount =
                    (*asset_amount).saturating_sub(MinLiquidity::get().saturated_into());
                let asset_in = match assets {
                    MarketType::Categorical(_) => {
                        Asset::CategoricalOutcome(market_id, idx.saturated_into())
                    }
                    MarketType::Scalar(_) => {
                        if idx == 0 {
                            Asset::ScalarOutcome(market_id, ScalarPosition::Long)
                        } else {
                            Asset::ScalarOutcome(market_id, ScalarPosition::Short)
                        }
                    }
                };

                if remaining_amount > zero_balance {
                    add_liqudity(remaining_amount, asset_in)?;
                }
            }

            let remaining_amount =
                (amount_base_asset).saturating_sub(MinLiquidity::get().saturated_into());

            if remaining_amount > zero_balance {
                add_liqudity(remaining_amount, Asset::Ztg)?;
            }

            Ok(Some(
                weight_bcs
                    .saturating_add(T::WeightInfo::deploy_swap_pool_for_market(weight_len))
                    .saturating_add(weight_pool_joins_and_sells)
                    .saturating_add(T::DbWeight::get().reads(2)),
            )
            .into())
        }

        #[pallet::weight(
            T::WeightInfo::deploy_swap_pool_for_market(weights.len() as u32)
        )]
        #[transactional]
        pub fn deploy_swap_pool_for_market(
            origin: OriginFor<T>,
            market_id: MarketIdOf<T>,
            weights: Vec<u128>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let market = T::MarketCommons::market(&market_id)?;
            ensure!(market.scoring_rule == ScoringRule::CPMM, Error::<T>::InvalidScoringRule);
            Self::ensure_market_is_active(&market.period)?;
            ensure!(market.status == MarketStatus::Active, Error::<T>::MarketIsNotActive);

            ensure!(T::MarketCommons::market_pool(&market_id).is_err(), Error::<T>::SwapPoolExists);

            let mut assets = Self::outcome_assets(market_id, &market);
            let base_asset = Asset::Ztg;
            assets.push(base_asset);

            let pool_id = T::Swaps::create_pool(
                sender,
                assets,
                base_asset,
                market_id,
                ScoringRule::CPMM,
                Some(Zero::zero()),
                Some(weights),
            )?;

            T::MarketCommons::insert_market_pool(market_id, pool_id);
            Ok(())
        }

        #[pallet::weight(10_000_000)]
        #[transactional]
        pub fn global_dispute(origin: OriginFor<T>, market_id: MarketIdOf<T>) -> DispatchResult {
            let _sender = ensure_signed(origin)?;
            let _market = T::MarketCommons::market(&market_id)?;
            Ok(())
        }

        #[pallet::weight(T::WeightInfo::redeem_shares_categorical()
            .max(T::WeightInfo::redeem_shares_scalar())
        )]
        #[transactional]
        pub fn redeem_shares(
            origin: OriginFor<T>,
            market_id: MarketIdOf<T>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            let market = T::MarketCommons::market(&market_id)?;
            let market_account = Self::market_account(market_id);

            ensure!(market.status == MarketStatus::Resolved, Error::<T>::MarketIsNotResolved);

            let resolved_outcome =
                market.resolved_outcome.ok_or(Error::<T>::MarketIsNotResolved)?;

            let winning_assets = match resolved_outcome {
                OutcomeReport::Categorical(category_index) => {
                    let winning_currency_id = Asset::CategoricalOutcome(market_id, category_index);
                    let winning_balance = T::Shares::free_balance(winning_currency_id, &sender);

                    ensure!(winning_balance > BalanceOf::<T>::zero(), Error::<T>::NoWinningBalance);

                    ensure!(
                        CurrencyOf::<T>::free_balance(&market_account) >= winning_balance,
                        Error::<T>::InsufficientFundsInMarketAccount,
                    );

                    vec![(winning_currency_id, winning_balance, winning_balance)]
                }
                OutcomeReport::Scalar(value) => {
                    let long_currency_id = Asset::ScalarOutcome(market_id, ScalarPosition::Long);
                    let short_currency_id = Asset::ScalarOutcome(market_id, ScalarPosition::Short);
                    let long_balance = T::Shares::free_balance(long_currency_id, &sender);
                    let short_balance = T::Shares::free_balance(short_currency_id, &sender);

                    ensure!(
                        long_balance > BalanceOf::<T>::zero()
                            || short_balance > BalanceOf::<T>::zero(),
                        Error::<T>::NoWinningBalance
                    );

                    let bound = if let MarketType::Scalar(range) = market.market_type {
                        range
                    } else {
                        return Err(Error::<T>::InvalidMarketType.into());
                    };

                    let calc_payouts = |final_value: u128,
                                        low: u128,
                                        high: u128|
                     -> (Perbill, Perbill) {
                        if final_value <= low {
                            return (Perbill::zero(), Perbill::one());
                        }
                        if final_value >= high {
                            return (Perbill::one(), Perbill::zero());
                        }

                        let payout_long: Perbill = Perbill::from_rational(
                            final_value.saturating_sub(low),
                            high.saturating_sub(low),
                        );
                        let payout_short: Perbill = Perbill::from_parts(
                            Perbill::one().deconstruct().saturating_sub(payout_long.deconstruct()),
                        );
                        (payout_long, payout_short)
                    };

                    let (long_percent, short_percent) =
                        calc_payouts(value, *bound.start(), *bound.end());

                    let long_payout = long_percent.mul_floor(long_balance);
                    let short_payout = short_percent.mul_floor(short_balance);
                    ensure!(
                        CurrencyOf::<T>::free_balance(&market_account)
                            >= long_payout + short_payout,
                        Error::<T>::InsufficientFundsInMarketAccount,
                    );

                    vec![
                        (long_currency_id, long_payout, long_balance),
                        (short_currency_id, short_payout, short_balance),
                    ]
                }
            };

            for (currency_id, payout, balance) in winning_assets {
                T::Shares::slash(currency_id, &sender, balance);

                let remaining_bal = CurrencyOf::<T>::free_balance(&market_account);
                let actual_payout = payout.min(remaining_bal);

                CurrencyOf::<T>::transfer(
                    &market_account,
                    &sender,
                    actual_payout,
                    ExistenceRequirement::AllowDeath,
                )?;
                if balance != <BalanceOf<T>>::zero() {
                    Self::deposit_event(Event::TokensRedeemed(
                        market_id,
                        currency_id,
                        balance,
                        actual_payout,
                        sender.clone(),
                    ));
                }
            }

            if let OutcomeReport::Categorical(_) = resolved_outcome {
                return Ok(Some(T::WeightInfo::redeem_shares_categorical()).into());
            } else if let OutcomeReport::Scalar(_) = resolved_outcome {
                return Ok(Some(T::WeightInfo::redeem_shares_scalar()).into());
            }

            Ok(None.into())
        }

        #[pallet::weight(T::WeightInfo::reject_market())]
        #[transactional]
        pub fn reject_market(origin: OriginFor<T>, market_id: MarketIdOf<T>) -> DispatchResult {
            T::ApprovalOrigin::ensure_origin(origin)?;

            let market = T::MarketCommons::market(&market_id)?;
            let creator = market.creator;
            let (imbalance, _) = CurrencyOf::<T>::slash_reserved_named(
                &RESERVE_ID,
                &creator,
                T::AdvisoryBond::get(),
            );
            T::Slash::on_unbalanced(imbalance);
            T::MarketCommons::remove_market(&market_id)?;
            Self::deposit_event(Event::MarketRejected(market_id));
            Self::deposit_event(Event::MarketDestroyed(market_id));
            Ok(())
        }

        #[pallet::weight(T::WeightInfo::report())]
        #[transactional]
        pub fn report(
            origin: OriginFor<T>,
            market_id: MarketIdOf<T>,
            outcome: OutcomeReport,
        ) -> DispatchResult {
            let sender = ensure_signed(origin.clone())?;

            let current_block = <frame_system::Pallet<T>>::block_number();
            let market_report = Report { at: current_block, by: sender.clone(), outcome };

            T::MarketCommons::mutate_market(&market_id, |market| {
                ensure!(market.report.is_none(), Error::<T>::MarketAlreadyReported);
                Self::ensure_market_is_closed(&market.period)?;
                Self::ensure_outcome_matches_market_type(market, &market_report.outcome)?;

                let mut should_check_origin = false;
                match market.period {
                    MarketPeriod::Block(ref range) => {
                        if current_block <= range.end + T::ReportingPeriod::get().into() {
                            should_check_origin = true;
                        }
                    }
                    MarketPeriod::Timestamp(ref range) => {
                        let rp_moment: MomentOf<T> = T::ReportingPeriod::get().into();
                        let reporting_period_in_ms = rp_moment * MILLISECS_PER_BLOCK.into();
                        if T::MarketCommons::now() <= range.end + reporting_period_in_ms {
                            should_check_origin = true;
                        }
                    }
                }

                if should_check_origin {
                    let sender_is_oracle = sender == market.oracle;
                    let origin_has_permission = T::ResolveOrigin::ensure_origin(origin).is_ok();
                    ensure!(
                        sender_is_oracle || origin_has_permission,
                        Error::<T>::ReporterNotOracle
                    );
                }

                market.report = Some(market_report.clone());
                market.status = MarketStatus::Reported;

                Ok(())
            })?;

            MarketIdsPerReportBlock::<T>::try_mutate(&current_block, |ids| {
                ids.try_push(market_id).map_err(|_| <Error<T>>::StorageOverflow)
            })?;

            Self::deposit_event(Event::MarketReported(
                market_id,
                MarketStatus::Reported,
                market_report,
            ));
            Ok(())
        }

        #[pallet::weight(
            T::WeightInfo::sell_complete_set(T::MaxCategories::get().into())
        )]
        #[transactional]
        pub fn sell_complete_set(
            origin: OriginFor<T>,
            market_id: MarketIdOf<T>,
            #[pallet::compact] amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ensure!(amount != BalanceOf::<T>::zero(), Error::<T>::ZeroAmount);

            let market = T::MarketCommons::market(&market_id)?;
            ensure!(market.scoring_rule == ScoringRule::CPMM, Error::<T>::InvalidScoringRule);
            Self::ensure_market_is_active(&market.period)?;
            ensure!(market.status == MarketStatus::Active, Error::<T>::MarketIsNotActive);

            let market_account = Self::market_account(market_id);
            ensure!(
                CurrencyOf::<T>::free_balance(&market_account) >= amount,
                "Market account does not have sufficient reserves.",
            );

            let assets = Self::outcome_assets(market_id, &market);

            for asset in assets.iter() {
                ensure!(
                    T::Shares::free_balance(*asset, &sender) >= amount,
                    Error::<T>::InsufficientShareBalance,
                );
            }

            for asset in assets.iter() {
                T::Shares::slash(*asset, &sender, amount);
            }

            CurrencyOf::<T>::transfer(
                &market_account,
                &sender,
                amount,
                ExistenceRequirement::AllowDeath,
            )?;

            Self::deposit_event(Event::SoldCompleteSet(market_id, amount, sender));
            let assets_len: u32 = assets.len().saturated_into();
            let max_cats: u32 = T::MaxCategories::get().into();
            Self::calculate_actual_weight(&T::WeightInfo::sell_complete_set, assets_len, max_cats)
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

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
        fn on_initialize(now: T::BlockNumber) -> Weight {
            let mut total_weight: Weight =
                Self::process_subsidy_collecting_markets(now, T::MarketCommons::now());

            with_transaction(|| {
                let output = Self::resolution_manager(now, |market_id, market| {
                    let weight = Self::on_resolution(market_id, market)?;
                    total_weight = total_weight.saturating_add(weight);
                    Ok(())
                });

                match output {
                    Err(err) => {
                        Self::deposit_event(Event::BadOnInitialize);
                        log::error!("Block {:?} was not initialized. Error: {:?}", now, err);
                        TransactionOutcome::Rollback(())
                    }
                    Ok(_) => TransactionOutcome::Commit(()),
                }
            });

            total_weight
        }
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::storage]
    pub type Disputes<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        MarketIdOf<T>,
        BoundedVec<MarketDispute<T::AccountId, T::BlockNumber>, T::MaxDisputes>,
        ValueQuery,
    >;

    #[pallet::storage]
    pub type MarketIdsPerDisputeBlock<T: Config> = StorageMap<
        _,
        Twox64Concat,
        T::BlockNumber,
        BoundedVec<MarketIdOf<T>, ConstU32<1024>>,
        ValueQuery,
    >;

    #[pallet::storage]
    pub type MarketIdsPerReportBlock<T: Config> = StorageMap<
        _,
        Twox64Concat,
        T::BlockNumber,
        BoundedVec<MarketIdOf<T>, ConstU32<1024>>,
        ValueQuery,
    >;

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

        fn clear_auto_resolve(market_id: &MarketIdOf<T>) -> DispatchResult {
            let market = T::MarketCommons::market(market_id)?;
            if market.status == MarketStatus::Reported {
                let report = market.report.ok_or(Error::<T>::MarketIsNotReported)?;
                MarketIdsPerReportBlock::<T>::mutate(&report.at, |ids| {
                    remove_item::<MarketIdOf<T>, _>(ids, market_id);
                });
            }
            if market.status == MarketStatus::Disputed {
                let disputes = Disputes::<T>::get(market_id);
                if let Some(last_dispute) = disputes.last() {
                    let at = last_dispute.at;
                    let mut old_disputes_per_block = MarketIdsPerDisputeBlock::<T>::get(&at);
                    remove_item::<MarketIdOf<T>, _>(&mut old_disputes_per_block, market_id);
                    MarketIdsPerDisputeBlock::<T>::mutate(&at, |ids| {
                        remove_item::<MarketIdOf<T>, _>(ids, market_id);
                    });
                }
            }

            Ok(())
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

        fn calculate_internal_resolve_weight(
            market: &Market<T::AccountId, T::BlockNumber, MomentOf<T>>,
            total_accounts: u32,
            total_asset_accounts: u32,
            total_categories: u32,
            total_disputes: u32,
        ) -> Weight {
            if let MarketType::Categorical(_) = market.market_type {
                if let MarketStatus::Reported = market.status {
                    T::WeightInfo::internal_resolve_categorical_reported(
                        total_accounts,
                        total_asset_accounts,
                        total_categories,
                    )
                } else {
                    T::WeightInfo::internal_resolve_categorical_disputed(
                        total_accounts,
                        total_asset_accounts,
                        total_categories,
                        total_disputes,
                    )
                }
            } else if let MarketStatus::Reported = market.status {
                T::WeightInfo::internal_resolve_scalar_reported()
            } else {
                T::WeightInfo::internal_resolve_scalar_disputed(total_disputes)
            }
        }

        fn ensure_can_not_dispute_the_same_outcome(
            disputes: &[MarketDispute<T::AccountId, T::BlockNumber>],
            report: &Report<T::AccountId, T::BlockNumber>,
            outcome: &OutcomeReport,
        ) -> DispatchResult {
            if let Some(last_dispute) = disputes.last() {
                ensure!(&last_dispute.outcome != outcome, Error::<T>::CannotDisputeSameOutcome);
            } else {
                ensure!(&report.outcome != outcome, Error::<T>::CannotDisputeSameOutcome);
            }
            Ok(())
        }

        #[inline]
        fn ensure_disputes_does_not_exceed_max_disputes(num_disputes: u32) -> DispatchResult {
            ensure!(num_disputes < T::MaxDisputes::get(), Error::<T>::MaxDisputesReached);
            Ok(())
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

        fn ensure_market_is_closed(
            period: &MarketPeriod<T::BlockNumber, MomentOf<T>>,
        ) -> DispatchResult {
            ensure!(
                match period {
                    MarketPeriod::Block(range) => {
                        <frame_system::Pallet<T>>::block_number() >= range.end
                    }
                    MarketPeriod::Timestamp(range) => {
                        T::MarketCommons::now() >= range.end
                    }
                },
                Error::<T>::MarketIsNotClosed
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

        fn ensure_outcome_matches_market_type(
            market: &Market<T::AccountId, T::BlockNumber, MomentOf<T>>,
            outcome: &OutcomeReport,
        ) -> DispatchResult {
            if let OutcomeReport::Categorical(ref inner) = outcome {
                if let MarketType::Categorical(ref categories) = market.market_type {
                    ensure!(inner < categories, Error::<T>::OutcomeOutOfRange);
                } else {
                    return Err(Error::<T>::OutcomeMismatch.into());
                }
            }
            if let OutcomeReport::Scalar(_) = outcome {
                ensure!(
                    matches!(&market.market_type, MarketType::Scalar(_)),
                    Error::<T>::OutcomeMismatch
                );
            }
            Ok(())
        }

        fn manage_resolved_categorical_market(
            market: &Market<T::AccountId, T::BlockNumber, MomentOf<T>>,
            market_id: &MarketIdOf<T>,
            outcome_report: &OutcomeReport,
        ) -> Result<[usize; 3], DispatchError> {
            let mut total_accounts: usize = 0;
            let mut total_asset_accounts: usize = 0;
            let mut total_categories: usize = 0;

            if let MarketType::Categorical(_) = market.market_type {
                if let OutcomeReport::Categorical(winning_asset_idx) = *outcome_report {
                    let assets = Self::outcome_assets(*market_id, market);
                    total_categories = assets.len().saturated_into();

                    let mut assets_iter = assets.iter().cloned();
                    let mut manage_asset = |asset: Asset<_>, winning_asset_idx| {
                        if let Asset::CategoricalOutcome(_, idx) = asset {
                            if idx == winning_asset_idx {
                                return 0;
                            }
                            let (total_accounts, accounts) =
                                T::Shares::accounts_by_currency_id(asset);
                            total_asset_accounts =
                                total_asset_accounts.saturating_add(accounts.len());
                            T::Shares::destroy_all(asset, accounts.iter().cloned());
                            total_accounts
                        } else {
                            0
                        }
                    };

                    if let Some(first_asset) = assets_iter.next() {
                        total_accounts = manage_asset(first_asset, winning_asset_idx);
                    }
                    for asset in assets_iter {
                        let _ = manage_asset(asset, winning_asset_idx);
                    }
                }
            }

            Ok([total_accounts, total_asset_accounts, total_categories])
        }

        fn on_resolution(
            market_id: &MarketIdOf<T>,
            market: &Market<T::AccountId, T::BlockNumber, MomentOf<T>>,
        ) -> Result<u64, DispatchError> {
            CurrencyOf::<T>::unreserve_named(&RESERVE_ID, &market.creator, T::ValidityBond::get());

            let mut total_weight = 0;
            let disputes = Disputes::<T>::get(market_id);

            let report = T::MarketCommons::report(market)?;

            let resolved_outcome = match market.status {
                MarketStatus::Reported => {
                    if report.by == market.oracle {
                        CurrencyOf::<T>::unreserve_named(
                            &RESERVE_ID,
                            &market.creator,
                            T::OracleBond::get(),
                        );
                    } else {
                        let (imbalance, _) = CurrencyOf::<T>::slash_reserved_named(
                            &RESERVE_ID,
                            &market.creator,
                            T::OracleBond::get(),
                        );

                        CurrencyOf::<T>::resolve_creating(&report.by, imbalance);
                    }

                    T::MarketCommons::report(market)?.outcome.clone()
                }
                MarketStatus::Disputed => {
                    let resolved_outcome = match market.mdm {
                        MarketDisputeMechanism::Authorized(_) => {
                            T::Authorized::on_resolution(&disputes, market_id, market)?
                        }
                        MarketDisputeMechanism::Court => {
                            T::Court::on_resolution(&disputes, market_id, market)?
                        }
                        MarketDisputeMechanism::SimpleDisputes => {
                            T::SimpleDisputes::on_resolution(&disputes, market_id, market)?
                        }
                    };

                    let mut correct_reporters: Vec<T::AccountId> = Vec::new();

                    let mut overall_imbalance = NegativeImbalanceOf::<T>::zero();

                    if report.outcome == resolved_outcome {
                        CurrencyOf::<T>::unreserve_named(
                            &RESERVE_ID,
                            &market.creator,
                            T::OracleBond::get(),
                        );
                    } else {
                        let (imbalance, _) = CurrencyOf::<T>::slash_reserved_named(
                            &RESERVE_ID,
                            &market.creator,
                            T::OracleBond::get(),
                        );

                        overall_imbalance.subsume(imbalance);
                    }

                    for (i, dispute) in disputes.iter().enumerate() {
                        let actual_bond = default_dispute_bond::<T>(i);
                        if dispute.outcome == resolved_outcome {
                            CurrencyOf::<T>::unreserve_named(&RESERVE_ID, &dispute.by, actual_bond);

                            correct_reporters.push(dispute.by.clone());
                        } else {
                            let (imbalance, _) = CurrencyOf::<T>::slash_reserved_named(
                                &RESERVE_ID,
                                &dispute.by,
                                actual_bond,
                            );
                            overall_imbalance.subsume(imbalance);
                        }
                    }

                    let reward_per_each = overall_imbalance
                        .peek()
                        .checked_div(&correct_reporters.len().saturated_into())
                        .ok_or(ArithmeticError::DivisionByZero)?;
                    for correct_reporter in &correct_reporters {
                        let (amount, leftover) = overall_imbalance.split(reward_per_each);
                        CurrencyOf::<T>::resolve_creating(correct_reporter, amount);
                        overall_imbalance = leftover;
                    }

                    resolved_outcome
                }
                _ => return Err(Error::<T>::InvalidMarketStatus.into()),
            };
            let to_stale_weight = Self::set_pool_to_stale(market, market_id, &resolved_outcome)?;
            total_weight = total_weight.saturating_add(to_stale_weight);
            T::LiquidityMining::distribute_market_incentives(market_id)?;

            let mut total_accounts = 0u32;
            let mut total_asset_accounts = 0u32;
            let mut total_categories = 0u32;

            if let Ok([local_total_accounts, local_total_asset_accounts, local_total_categories]) =
                Self::manage_resolved_categorical_market(market, market_id, &resolved_outcome)
            {
                total_accounts = local_total_accounts.saturated_into();
                total_asset_accounts = local_total_asset_accounts.saturated_into();
                total_categories = local_total_categories.saturated_into();
            }

            T::MarketCommons::mutate_market(market_id, |m| {
                m.status = MarketStatus::Resolved;
                m.resolved_outcome = Some(resolved_outcome.clone());
                Ok(())
            })?;
            Self::deposit_event(Event::MarketResolved(
                *market_id,
                MarketStatus::Resolved,
                resolved_outcome,
            ));
            Ok(total_weight.saturating_add(Self::calculate_internal_resolve_weight(
                market,
                total_accounts,
                total_asset_accounts,
                total_categories,
                disputes.len().saturated_into(),
            )))
        }

        pub(crate) fn process_subsidy_collecting_markets(
            current_block: T::BlockNumber,
            current_time: MomentOf<T>,
        ) -> Weight {
            let mut total_weight = 0;
            let dbweight = T::DbWeight::get();
            let one_read = T::DbWeight::get().reads(1);
            let one_write = T::DbWeight::get().writes(1);

            let retain_closure =
                |subsidy_info: &SubsidyUntil<T::BlockNumber, MomentOf<T>, MarketIdOf<T>>| {
                    let market_ready = match &subsidy_info.period {
                        MarketPeriod::Block(period) => period.start <= current_block,
                        MarketPeriod::Timestamp(period) => period.start <= current_time,
                    };

                    if market_ready {
                        let pool_id = T::MarketCommons::market_pool(&subsidy_info.market_id);
                        total_weight.saturating_add(one_read);

                        if let Err(err) = pool_id {
                            log::error!(
                                "[PredictionMarkets] Cannot find pool associated to market.
                            market_id: {:?}, error: {:?}",
                                pool_id,
                                err
                            );
                            return true;
                        }

                        let end_subsidy_result =
                            T::Swaps::end_subsidy_phase(pool_id.unwrap_or(u128::MAX));

                        if let Ok(result) = end_subsidy_result {
                            total_weight = total_weight.saturating_add(result.weight);

                            if result.result {
                                let mutate_result =
                                    T::MarketCommons::mutate_market(&subsidy_info.market_id, |m| {
                                        m.status = MarketStatus::Active;
                                        Ok(())
                                    });

                                total_weight =
                                    total_weight.saturating_add(one_read).saturating_add(one_write);

                                if let Err(err) = mutate_result {
                                    log::error!(
                                        "[PredictionMarkets] Cannot find market associated to \
                                         market id.
                                    market_id: {:?}, error: {:?}",
                                        subsidy_info.market_id,
                                        err
                                    );
                                    return true;
                                }

                                Self::deposit_event(Event::MarketStartedWithSubsidy(
                                    subsidy_info.market_id,
                                    MarketStatus::Active,
                                ));
                            } else {
                                let destroy_result = T::Swaps::destroy_pool_in_subsidy_phase(
                                    pool_id.unwrap_or(u128::MAX),
                                );

                                if let Err(err) = destroy_result {
                                    log::error!(
                                        "[PredictionMarkets] Cannot destroy pool with missing \
                                         subsidy.
                                    market_id: {:?}, error: {:?}",
                                        subsidy_info.market_id,
                                        err
                                    );
                                    return true;
                                } else if let Ok(weight) = destroy_result {
                                    total_weight = total_weight.saturating_add(weight);
                                }

                                let market_result =
                                    T::MarketCommons::mutate_market(&subsidy_info.market_id, |m| {
                                        m.status = MarketStatus::InsufficientSubsidy;

                                        if m.creation == MarketCreation::Permissionless {
                                            let required_bond =
                                                T::ValidityBond::get() + T::OracleBond::get();
                                            CurrencyOf::<T>::unreserve_named(
                                                &RESERVE_ID,
                                                &m.creator,
                                                required_bond,
                                            );
                                        } else if m.creation == MarketCreation::Advised {
                                            CurrencyOf::<T>::unreserve_named(
                                                &RESERVE_ID,
                                                &m.creator,
                                                T::OracleBond::get(),
                                            );
                                        }

                                        total_weight = total_weight
                                            .saturating_add(dbweight.reads(2))
                                            .saturating_add(dbweight.writes(2));
                                        Ok(())
                                    });

                                if let Err(err) = market_result {
                                    log::error!(
                                        "[PredictionMarkets] Cannot find market associated to \
                                         market id.
                                    market_id: {:?}, error: {:?}",
                                        subsidy_info.market_id,
                                        err
                                    );
                                    return true;
                                }

                                let _ =
                                    T::MarketCommons::remove_market_pool(&subsidy_info.market_id);
                                total_weight =
                                    total_weight.saturating_add(one_read).saturating_add(one_write);
                                Self::deposit_event(Event::MarketInsufficientSubsidy(
                                    subsidy_info.market_id,
                                    MarketStatus::InsufficientSubsidy,
                                ));
                            }

                            return false;
                        } else if let Err(err) = end_subsidy_result {
                            log::error!(
                                "[PredictionMarkets] An error occured during end of subsidy phase.
                        pool_id: {:?}, market_id: {:?}, error: {:?}",
                                pool_id,
                                subsidy_info.market_id,
                                err
                            );
                        }
                    }

                    true
                };

            let mut weight_basis = 0;
            <MarketsCollectingSubsidy<T>>::mutate(
                |e: &mut BoundedVec<
                    SubsidyUntil<T::BlockNumber, MomentOf<T>, MarketIdOf<T>>,
                    _,
                >| {
                    weight_basis = T::WeightInfo::process_subsidy_collecting_markets_raw(
                        e.len().saturated_into(),
                    );
                    e.retain(retain_closure);
                },
            );

            weight_basis.saturating_add(total_weight)
        }

        fn remove_last_dispute_from_market_ids_per_dispute_block(
            disputes: &[MarketDispute<T::AccountId, T::BlockNumber>],
            market_id: &MarketIdOf<T>,
        ) -> DispatchResult {
            if let Some(last_dispute) = disputes.last() {
                let at = last_dispute.at;
                MarketIdsPerDisputeBlock::<T>::mutate(&at, |ids| {
                    remove_item::<MarketIdOf<T>, _>(ids, market_id);
                });
            }
            Ok(())
        }

        fn resolution_manager<F>(now: T::BlockNumber, mut cb: F) -> DispatchResult
        where
            F: FnMut(
                &MarketIdOf<T>,
                &Market<T::AccountId, T::BlockNumber, MomentOf<T>>,
            ) -> DispatchResult,
        {
            let dispute_period = T::DisputePeriod::get();
            if now <= dispute_period {
                return Ok(());
            }

            let block = now.saturating_sub(dispute_period);

            for id in MarketIdsPerReportBlock::<T>::get(&block).iter() {
                let market = T::MarketCommons::market(id)?;
                if let MarketStatus::Reported = market.status {
                    cb(id, &market)?;
                }
            }

            for id in MarketIdsPerDisputeBlock::<T>::get(&block).iter() {
                let market = T::MarketCommons::market(id)?;
                cb(id, &market)?;
            }

            Ok(())
        }

        fn set_market_as_disputed(
            market: &Market<T::AccountId, T::BlockNumber, MomentOf<T>>,
            market_id: &MarketIdOf<T>,
        ) -> DispatchResult {
            if market.status != MarketStatus::Disputed {
                T::MarketCommons::mutate_market(market_id, |m| {
                    m.status = MarketStatus::Disputed;
                    Ok(())
                })?;
            }
            Ok(())
        }

        fn set_pool_to_stale(
            market: &Market<T::AccountId, T::BlockNumber, MomentOf<T>>,
            market_id: &MarketIdOf<T>,
            outcome_report: &OutcomeReport,
        ) -> Result<Weight, DispatchError> {
            let pool_id = if let Ok(el) = T::MarketCommons::market_pool(market_id) {
                el
            } else {
                return Ok(T::DbWeight::get().reads(1));
            };
            let market_account = Self::market_account(*market_id);
            let weight = T::Swaps::set_pool_as_stale(
                &market.market_type,
                pool_id,
                outcome_report,
                &market_account,
            )?;
            Ok(weight.saturating_add(T::DbWeight::get().reads(2)))
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

        fn validate_dispute(
            disputes: &[MarketDispute<T::AccountId, T::BlockNumber>],
            market: &Market<T::AccountId, T::BlockNumber, MomentOf<T>>,
            num_disputes: u32,
            outcome: &OutcomeReport,
        ) -> DispatchResult {
            ensure!(market.report.is_some(), Error::<T>::MarketNotReported);
            Self::ensure_outcome_matches_market_type(market, outcome)?;
            Self::ensure_can_not_dispute_the_same_outcome(
                disputes,
                (&market.report.as_ref()).ok_or(Error::<T>::MarketNotReported)?,
                outcome,
            )?;
            Self::ensure_disputes_does_not_exceed_max_disputes(num_disputes)?;
            Ok(())
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

    fn remove_item<I: cmp::PartialEq, G>(items: &mut BoundedVec<I, G>, item: &I) {
        if let Some(pos) = items.iter().position(|i| i == item) {
            items.swap_remove(pos);
        }
    }
}
