This crate provides the `Thin` type, a 1-pointer-wide trait object that also aims to be ffi-safe.

 ```rust
 use thin_trait_objects::*;

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

    // the inner value can be obtained via downcasting
    let value: u8 = unsafe { thin.downcast() };
    assert_eq!(value, 9u8);
}
 ```

 Limitations:
 - Annotated traits must have a `'static` bound (for now).
 - Methods with non-lifetime generics are not supported.