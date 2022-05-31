#![cfg(test)]

use crate::{
    market_commons_pallet_api::MarketCommonsPalletApi,
    mock::{ExtBuilder, MarketCommons, Runtime},
    MarketCounter, Markets,
};
use frame_support::{assert_err, assert_noop, assert_ok};
use sp_runtime::DispatchError;
use zeitgeist_primitives::types::{
    AccountIdTest, BlockNumber, Market, MarketCreation, MarketDisputeMechanism, MarketPeriod,
    MarketStatus, MarketType, Moment, ScoringRule,
};

const MARKET_DUMMY: Market<AccountIdTest, BlockNumber, Moment> = Market {
    creation: MarketCreation::Permissionless,
    creator_fee: 0,
    creator: 0,
    market_type: MarketType::Scalar(0..=100),
    mdm: MarketDisputeMechanism::Authorized(0),
    metadata: vec![],
    oracle: 0,
    period: MarketPeriod::Block(0..100),
    report: None,
    resolved_outcome: None,
    scoring_rule: ScoringRule::CPMM,
    status: MarketStatus::Disputed,
};

#[test]
fn latest_market_id_interacts_correctly_with_push_market() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(MarketCommons::push_market(market_mock(0)));
        assert_eq!(MarketCommons::latest_market_id().unwrap(), 0);
        assert_ok!(MarketCommons::push_market(market_mock(1)));
        assert_eq!(MarketCommons::latest_market_id().unwrap(), 1);
        assert_ok!(MarketCommons::push_market(market_mock(1)));
        assert_eq!(MarketCommons::latest_market_id().unwrap(), 2);
    });
}

#[test]
fn latest_market_id_fails_if_there_are_no_markets() {
    ExtBuilder::default().build().execute_with(|| {
        assert_err!(
            MarketCommons::latest_market_id(),
            crate::Error::<Runtime>::NoMarketHasBeenCreated
        );
    });
}

#[test]
fn market_interacts_correctly_with_push_market() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(MarketCommons::push_market(market_mock(0)));
        assert_ok!(MarketCommons::push_market(market_mock(1)));
        assert_ok!(MarketCommons::push_market(market_mock(2)));
        assert_eq!(MarketCommons::market(&0).unwrap().oracle, 0);
        assert_eq!(MarketCommons::market(&1).unwrap().oracle, 1);
        assert_eq!(MarketCommons::market(&2).unwrap().oracle, 2);
    });
}

#[test]
fn markets_interacts_correctly_with_push_market() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(MarketCommons::push_market(market_mock(0)));
        assert_ok!(MarketCommons::push_market(market_mock(1)));
        assert_ok!(MarketCommons::push_market(market_mock(2)));
        assert_eq!(<Markets<Runtime>>::get(0).unwrap().oracle, 0);
        assert_eq!(<Markets<Runtime>>::get(1).unwrap().oracle, 1);
        assert_eq!(<Markets<Runtime>>::get(2).unwrap().oracle, 2);
    });
}

#[test]
fn market_fails_if_market_does_not_exist() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(MarketCommons::market(&0), crate::Error::<Runtime>::MarketDoesNotExist);
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_noop!(MarketCommons::market(&3), crate::Error::<Runtime>::MarketDoesNotExist);
    });
}

#[test]
fn mutate_market_succeeds_if_closure_succeeds() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(MarketCommons::push_market(market_mock(0)));
        assert_ok!(MarketCommons::mutate_market(&0, |market| {
            market.oracle = 1;
            Ok(())
        }));
        assert_eq!(MarketCommons::market(&0).unwrap().oracle, 1);
    });
}

#[test]
fn mutate_market_fails_if_market_does_not_exist() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            MarketCommons::mutate_market(&0, |_| Ok(())),
            crate::Error::<Runtime>::MarketDoesNotExist
        );
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_noop!(
            MarketCommons::mutate_market(&3, |_| Ok(())),
            crate::Error::<Runtime>::MarketDoesNotExist
        );
    });
}

#[test]
fn mutate_market_is_noop_if_closure_fails() {
    ExtBuilder::default().build().execute_with(|| {
        let err = DispatchError::Other("foo");
        assert_ok!(MarketCommons::push_market(market_mock(0)));
        assert_noop!(
            // We change the market to check that `mutate_market` is no-op when it errors.
            MarketCommons::mutate_market(&0, |market| {
                market.oracle = 1;
                Err(err)
            }),
            err
        );
    });
}

#[test]
fn remove_market_correctly_interacts_with_push_market() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(MarketCommons::push_market(market_mock(0)));
        assert_ok!(MarketCommons::push_market(market_mock(1)));
        assert_ok!(MarketCommons::push_market(market_mock(2)));

        assert_ok!(MarketCommons::remove_market(&1));
        assert_eq!(MarketCommons::market(&0).unwrap().oracle, 0);
        assert_noop!(MarketCommons::market(&1), crate::Error::<Runtime>::MarketDoesNotExist);
        assert_eq!(MarketCommons::market(&2).unwrap().oracle, 2);

        assert_ok!(MarketCommons::remove_market(&2));
        assert_eq!(MarketCommons::market(&0).unwrap().oracle, 0);
        assert_noop!(MarketCommons::market(&1), crate::Error::<Runtime>::MarketDoesNotExist);
        assert_noop!(MarketCommons::market(&2), crate::Error::<Runtime>::MarketDoesNotExist);

        assert_ok!(MarketCommons::remove_market(&0));
        assert_noop!(MarketCommons::market(&0), crate::Error::<Runtime>::MarketDoesNotExist);
        assert_noop!(MarketCommons::market(&1), crate::Error::<Runtime>::MarketDoesNotExist);
        assert_noop!(MarketCommons::market(&2), crate::Error::<Runtime>::MarketDoesNotExist);
    });
}

#[test]
fn remove_market_fails_if_market_does_not_exist() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(MarketCommons::remove_market(&0), crate::Error::<Runtime>::MarketDoesNotExist);
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_noop!(MarketCommons::remove_market(&3), crate::Error::<Runtime>::MarketDoesNotExist);
    });
}

#[test]
fn insert_market_pool_fails_if_market_does_not_exist() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            MarketCommons::insert_market_pool(0, 15),
            crate::Error::<Runtime>::MarketDoesNotExist
        );
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_noop!(
            MarketCommons::insert_market_pool(3, 12),
            crate::Error::<Runtime>::MarketDoesNotExist
        );
    });
}

#[test]
fn insert_market_pool_fails_if_market_has_a_pool() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::insert_market_pool(0, 15));
        assert_noop!(
            MarketCommons::insert_market_pool(0, 14),
            crate::Error::<Runtime>::DuplicatePool
        );
    });
}

#[test]
fn market_pool_correctly_interacts_with_insert_market_pool() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::insert_market_pool(0, 15));
        assert_ok!(MarketCommons::insert_market_pool(1, 14));
        assert_ok!(MarketCommons::insert_market_pool(2, 13));
        assert_eq!(MarketCommons::market_pool(&0).unwrap(), 15);
        assert_eq!(MarketCommons::market_pool(&1).unwrap(), 14);
        assert_eq!(MarketCommons::market_pool(&2).unwrap(), 13);
    });
}

#[test]
fn market_pool_fails_if_market_has_no_pool() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            MarketCommons::market_pool(&0),
            crate::Error::<Runtime>::MarketPoolDoesNotExist
        );
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::insert_market_pool(0, 15));
        assert_ok!(MarketCommons::insert_market_pool(1, 14));
        assert_ok!(MarketCommons::insert_market_pool(2, 13));
        assert_noop!(
            MarketCommons::market_pool(&3),
            crate::Error::<Runtime>::MarketPoolDoesNotExist
        );
    });
}

#[test]
fn remove_market_pool_correctly_interacts_with_insert_market_pool() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::insert_market_pool(0, 15));
        assert_ok!(MarketCommons::insert_market_pool(1, 14));
        assert_ok!(MarketCommons::insert_market_pool(2, 13));

        assert_ok!(MarketCommons::remove_market_pool(&1));
        assert_eq!(MarketCommons::market_pool(&0).unwrap(), 15);
        assert_noop!(
            MarketCommons::market_pool(&1),
            crate::Error::<Runtime>::MarketPoolDoesNotExist
        );
        assert_eq!(MarketCommons::market_pool(&2).unwrap(), 13);

        assert_ok!(MarketCommons::remove_market_pool(&2));
        assert_eq!(MarketCommons::market_pool(&0).unwrap(), 15);
        assert_noop!(
            MarketCommons::market_pool(&1),
            crate::Error::<Runtime>::MarketPoolDoesNotExist
        );
        assert_noop!(
            MarketCommons::market_pool(&2),
            crate::Error::<Runtime>::MarketPoolDoesNotExist
        );

        assert_ok!(MarketCommons::remove_market_pool(&0));
        assert_noop!(
            MarketCommons::market_pool(&0),
            crate::Error::<Runtime>::MarketPoolDoesNotExist
        );
        assert_noop!(
            MarketCommons::market_pool(&1),
            crate::Error::<Runtime>::MarketPoolDoesNotExist
        );
        assert_noop!(
            MarketCommons::market_pool(&2),
            crate::Error::<Runtime>::MarketPoolDoesNotExist
        );
    });
}

#[test]
fn remove_market_pool_fails_if_market_has_no_pool() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            MarketCommons::remove_market_pool(&0),
            crate::Error::<Runtime>::MarketPoolDoesNotExist
        );
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_ok!(MarketCommons::insert_market_pool(0, 15));
        assert_ok!(MarketCommons::insert_market_pool(1, 14));
        assert_ok!(MarketCommons::insert_market_pool(2, 13));
        assert_noop!(
            MarketCommons::remove_market_pool(&3),
            crate::Error::<Runtime>::MarketPoolDoesNotExist
        );
    });
}

#[test]
fn market_counter_interacts_correctly_with_push_market_and_remove_market() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(<MarketCounter<Runtime>>::get(), 0);
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_eq!(<MarketCounter<Runtime>>::get(), 1);
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_eq!(<MarketCounter<Runtime>>::get(), 2);
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_eq!(<MarketCounter<Runtime>>::get(), 3);
        assert_ok!(MarketCommons::remove_market(&1));
        assert_eq!(<MarketCounter<Runtime>>::get(), 3);
        assert_ok!(MarketCommons::remove_market(&2));
        assert_eq!(<MarketCounter<Runtime>>::get(), 3);
        assert_ok!(MarketCommons::push_market(MARKET_DUMMY));
        assert_eq!(<MarketCounter<Runtime>>::get(), 4);
    });
}

fn market_mock(
    id: AccountIdTest,
) -> zeitgeist_primitives::types::Market<AccountIdTest, BlockNumber, Moment> {
    let mut market = MARKET_DUMMY;
    market.oracle = id;
    market
}
