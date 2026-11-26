/// Package version and the target used to build this binary.
pub(crate) const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("JDA_BUILD_TARGET"),
    ")"
);
