use std::rc::Rc;
use crate::prelude::*;

// TODO: more standard types

impl_stable_any! {
    Option<T>;
    Result<T, E>;
    Rc<T>
}