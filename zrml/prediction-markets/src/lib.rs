pub enum E {
    S([u8; 50]),
}

pub fn f() {
    let x = E::S([0u8; 50]);
    || {
        let E::S(_x) = x;
    };
}
