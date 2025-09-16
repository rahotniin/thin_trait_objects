This crate provides the `Thin` type, an ffi-safe 1-pointer-wide trait object, used like so:

```rust
    use thin_trait_objects::*;
    
    #[thin]
    trait Foo {
        fn foo(&self) -> u8;
    }
    
    impl Foo for u8 {
        fn foo(&self) -> u8 {
            *self
        }
    }
    
    fn main() {
        let thin = Thin::<dyn Foo>::new(8u8);
        // `Foo` is automatically implemented for `Thin<dyn Foo>`
        assert_eq!(thin.foo(), 8u8);
    }
```