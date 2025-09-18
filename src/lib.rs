//! This crate provides the `Thin` type, a 1-pointer-wide trait object that also aims to be ffi-safe.Used like so:
//!
//! ```rust
//! use thin_trait_objects::*;
//!
//! #[thin]
//! trait Foo: 'static {
//!     fn get(&self) -> &u8;
//!     fn add(&mut self, other: u8);
//! }
//!
//! impl Foo for u8 {
//!     fn get(&self) -> &u8 {
//!         self
//!     }
//!
//!     fn add(&mut self, other: u8) {
//!         *self += other
//!     }
//! }
//!
//!
//! let mut thin = Thin::<dyn Foo>::new(8u8);
//! // `Foo` is automatically implemented for `Thin<dyn Foo>`
//! thin.add(1u8);
//! assert_eq!(*thin.get(), 9u8);
//!
//! // the inner value can be obtained via downcasting
//! let value: u8 = unsafe { thin.downcast() };
//! assert_eq!(value, 9u8);
//!
//! ```
//!
//! Limitations:
//! - Annotated traits must have a `'static` bound (for now).
//! - Methods with non-lifetime generics are not supported.

use std::marker::PhantomData;
use std::ptr::NonNull;
pub use thin_trait_objects_macros::thin;

#[repr(transparent)]
pub struct Thin<T: ?Sized> {
    // type-erased `*mut Bundle<K> where `K: F` and `T` is `dyn F`
    pub ptr: NonNull<()>,
    phantom: PhantomData<T>,
}

impl<T: ?Sized> Thin<T> {
    #[doc(hidden)]
    pub unsafe fn from_raw(ptr: *mut ()) -> Thin<T> {
        Thin {
            ptr: NonNull::new(ptr).unwrap(),
            phantom: PhantomData
        }
    }
}

pub trait ThinExt<T: ?Sized, K> {
    /// Creates a new `Thin<dyn _>` from the given value.
    fn new(val: K) -> Self;

    /// Consumes this `Thin` and returns the inner value.
    ///
    /// # Safety
    /// The contained value must be of type K. Calling this method with the incorrect type is undefined behavior.
    unsafe fn downcast(self) -> K;
}

impl<T: ?Sized> Drop for Thin<T> {
    fn drop(&mut self) {
        // SAFETY: `Bundle` and `VTable` are `#[repr(C)]`,
        // so the `drop` field of `VTable` will be positioned first in the memory layout of `Bundle`.
        // see: https://adventures.michaelfbryan.com/posts/ffi-safe-polymorphism-in-rust/?utm_source=user-forums&utm_medium=social&utm_campaign=thin-trait-objects#pointer-to-vtable--object
        let dropper: extern "C" fn(*mut ()) = unsafe { *self.ptr.as_ptr().cast() };
        dropper(self.ptr.as_ptr());
    }
}

//========================//
// Type erasure for the 'receivers' of shims functions

#[repr(transparent)]
pub struct RefSelf<'a> {
    pub ptr: *const (),
    marker: PhantomData<&'a ()>,
}

impl<'a> RefSelf<'a> {
    pub fn new<T: ?Sized>(thin: &'a Thin<T>) -> Self {
        Self {
            ptr: thin.ptr.as_ptr(),
            marker: PhantomData,
        }
    }
}

#[repr(C)]
pub struct MutSelf<'a> {
    pub ptr: *mut (),
    marker: PhantomData<&'a ()>,
}

impl<'a> MutSelf<'a> {
    pub fn new<T: ?Sized>(thin: &'a mut Thin<T>) -> MutSelf<'a> {
        MutSelf {
            ptr: thin.ptr.as_ptr(),
            marker: PhantomData,
        }
    }
}

//========================//

#[cfg(test)]
mod tests {
    use super::*;

    #[thin]
    trait Foo: 'static {
        fn add(&mut self, other: u8);
        fn get(&self) -> &u8;
    }

    impl Foo for u8 {
        fn add(&mut self, other: u8) {
            *self += other;
        }
        fn get(&self) -> &u8 {
            self
        }
    }

    #[test]
    fn one_pointer_wide() {
        assert_eq!(size_of::<Thin<dyn Foo>>(), size_of::<usize>());
    }

    #[test]
    fn niche_optimisations() {
        assert_eq!(size_of::<Option<Thin<dyn Foo>>>(), size_of::<usize>());
    }

    #[test]
    fn thin() {
        let mut thin = Thin::<dyn Foo>::new(8u8);
        thin.add(1u8);
        assert_eq!(*thin.get(), 9u8);

        let value: u8 = unsafe { thin.downcast() };
        assert_eq!(value, 9u8);
    }
}

/// Example output of the `#[thin]` attribute
mod example_macro_expansion {
    use super::*;

    // #[thin]
    trait Foo: 'static {
        fn add(&mut self, other: u8);
        fn get(&self) -> &u8;
    }

    // expansion:
    const _: () = {
        #[repr(C)]
        struct VTable {
            drop: extern "C" fn(*mut ()),
            add: extern "C" fn(MutSelf<'_>, u8),
            get: extern "C" fn(RefSelf<'_>) -> &'_ u8,
        }
        extern "C" fn drop<T: Foo>(ptr: *mut ()) {
            let bundle = ptr as *mut Bundle<T>;
            let _ = unsafe { Box::from_raw(bundle) };
        }
        extern "C" fn add<T: Foo>(recv: MutSelf<'_>, other: u8) {
            let bundle = unsafe { &mut *(recv.ptr as *mut Bundle<T>) };
            let recv = &mut bundle.value;
            T::add(recv, other)
        }
        extern "C" fn get<T: Foo>(recv: RefSelf<'_>) -> &'_ u8 {
            let bundle = unsafe { &*(recv.ptr as *const Bundle<T>) };
            let recv = &bundle.value;
            T::get(recv)
        }
        #[repr(C)]
        struct Bundle<T: Foo> {
            vtable: VTable,
            value: T,
        }
        impl<T: Foo> ThinExt<dyn Foo, T> for Thin<dyn Foo> {
            fn new(value: T) -> Self {
                let vtable = VTable { drop: drop::<T>, add: add::<T>, get: get::<T> };
                let bundle = Bundle { vtable, value };
                let ptr = Box::into_raw(Box::new(bundle));
                unsafe { Thin::from_raw(ptr as *mut ()) }
            }
            unsafe fn downcast(self) -> T {
                let ptr = self.ptr.as_ptr() as *mut Bundle<T>;
                ::std::mem::forget(self);
                let bundle = unsafe { Box::from_raw(ptr) };
                bundle.value
            }
        }
        impl Foo for Thin<dyn Foo> {
            fn add(&mut self, other: u8) {
                let shim = {
                    let vtable = unsafe { &*(self.ptr.as_ptr() as *const VTable) };
                    vtable.add
                };
                let recv = MutSelf::new(self);
                shim(recv, other)
            }
            fn get(&self) -> &'_ u8 {
                let shim = {
                    let vtable = unsafe { &*(self.ptr.as_ptr() as *const VTable) };
                    vtable.get
                };
                let recv = RefSelf::new(self);
                shim(recv)
            }
        }
    };
}