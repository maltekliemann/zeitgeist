use frame_support::storage::{with_transaction, TransactionOutcome};

pub enum MultiHash {
    Sha3_384([u8; 50]),
}

pub fn create_categorical_market(metadata: MultiHash) -> Option<u128> {
    with_transaction(|| {
        let MultiHash::Sha3_384(_multihash) = metadata;
        TransactionOutcome::Commit(Some(0))
    })
}
