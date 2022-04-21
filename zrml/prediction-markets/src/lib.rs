pub fn with_transaction(f: impl FnOnce() -> i32) -> i32 {
    f()
}

pub enum MultiHash {
    Sha3_384([u8; 50]),
}

pub fn create_categorical_market(metadata: MultiHash) -> i32 {
    with_transaction(|| {
        let MultiHash::Sha3_384(_multihash) = metadata;
        0i32
    })
}
