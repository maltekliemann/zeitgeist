fn g(_: impl FnOnce()) { }

pub enum E {
    S([u8; 50]),
}

pub fn f(x: E) {
    g(|| {
        let E::S(_x) = x;
    });
}
