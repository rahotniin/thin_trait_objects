# thin_trait_objects
This crate provides the `#[thin]` attribute and `Thin` type for creating 1-pointer-wide trait objects. 

This project was inspired by the similarly named [thin_trait_object](https://crates.io/crates/thin_trait_object)
crate, which describes (far better than I could) the relevant motivations and use-cases. The implementation is
based on [this](https://adventures.michaelfbryan.com/posts/ffi-safe-polymorphism-in-rust/) article by Michael-F-Bryan.

#### Example Usage
```rust
use thin_trait_objects::prelude::*;

#[thin]
trait Foo: 'static {
    fn get(&self) -> &u8;
    fn add(&mut self, other: u8);
}

impl Foo for u8 {
    fn get(&self) -> &u8 {
        self
    }
    fn add(&mut self, other: u8) {
        *self += other
    }
}

fn main() {
    let mut thin = Thin::<dyn Foo>::new(8u8);
    // `Foo` is automatically implemented for `Thin<dyn Foo>`
    thin.add(1u8);
    assert_eq!(*thin.get(), 9u8);
}
```

#### Built-in support for `Any`
`Thin<dyn Any>` is provided as a replacement for `Box<dyn Any>`.

```rust
use std::any::Any;
use thin_trait_objects::prelude::*;

fn main() {
    let mut thin = Thin::<dyn Any>::new(8u8);
    let borrow = thin.downcast_mut::<u8>().unwrap();
    *borrow += 1;
    assert_eq!(thin.downcast::<u8>(), Some(9u8));
}
```

#### FFI
The `Thin` type is designed to be FFI-safe, with a focus on rust-to-rust via the C ABI.

However, this doesn't apply to `Thin<dyn Any>`, as `TypeId`s aren't guaranteed to be the same 
between different compilations. The `StableAny` trait is provided as a work-around, and may be
implemented for your types using `#[derive(StableAny)]` or the `impl_stable_any!` macro.

#### Limitations
- Annotated traits must have a `'static` bound (for now).
- Methods with non-lifetime generics are not supported.