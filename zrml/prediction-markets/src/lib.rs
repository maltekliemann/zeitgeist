#[cfg(feature = "arbitrary")]
use arbitrary::{Arbitrary, Result, Unstructured};
use frame_support::dispatch::{Decode, Encode};

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum MultiHash {
    Sha3_384([u8; 50]),
}

pub fn create_categorical_market(
    metadata: MultiHash,
) -> Result<Option<u128>, &'static str> {
    use frame_support::storage::{with_transaction, TransactionOutcome};
    with_transaction(|| {
        let r = (|| {
            let MultiHash::Sha3_384(_multihash) = metadata;
            Ok(Some(0))
        })();
        if r.is_ok() {
            TransactionOutcome::Commit(r)
        } else {
            TransactionOutcome::Rollback(r)
        }
    })
}
