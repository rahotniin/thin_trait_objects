use crate::prelude::*;

// TODO: change `impl_uuid` not to use `module_path` and the cargo package version,
//  as they refer to this crate and not the one who's types we're implementing UUID for.

impl_uuid! {
    struct char;
    struct u8;
    struct u16;
    enum Option<T> {}
}