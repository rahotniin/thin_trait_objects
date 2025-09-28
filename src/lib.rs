//! # thin_trait_objects
//! This crate provides the `#[thin]` attribute and `Thin` type for creating 1-pointer-wide trait objects.
//!
//!
//! This project was inspired by the similarly named [thin_trait_object](https://crates.io/crates/thin_trait_object)
//! crate, which describes (far better than I could) the relevant motivations and use-cases. The implementation is
//! based on [this](https://adventures.michaelfbryan.com/posts/ffi-safe-polymorphism-in-rust/) article by Michael-F-Bryan.
//!
//! #### Example Usage
//! ```rust
//! use thin_trait_objects::prelude::*;
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
//!     fn add(&mut self, other: u8) {
//!         *self += other
//!     }
//! }
//!
//! fn main() {
//!     let mut thin = Thin::<dyn Foo>::new(8u8);
//!     // `Foo` is automatically implemented for `Thin<dyn Foo>`
//!     thin.add(1u8);
//!     assert_eq!(*thin.get(), 9u8);
//! }
//! ```
//!
//! #### Built-in support for `Any`
//! `Thin<dyn Any>` is provided as a replacement for `Box<dyn Any>`.
//!
//! ```rust
//! use std::any::Any;
//! use thin_trait_objects::prelude::*;
//!
//! fn main() {
//!     let mut thin = Thin::<dyn Any>::new(8u8);
//!     let borrow = thin.downcast_mut::<u8>().unwrap();
//!     *borrow += 1;
//!     assert_eq!(thin.downcast::<u8>(), Some(9u8));
//! }
//! ```
//!
//! #### FFI
//! The `Thin` type is designed to be FFI-safe, with a focus on rust-to-rust via the C ABI.
//!
//! However, this doesn't apply to `Thin<dyn Any>`, as `TypeId`s aren't guaranteed to be the same
//! between different compilations. The `StableAny` trait is provided as a work-around, and may be
//! implemented for your types using `#[derive(StableAny)]` or the `impl_stable_any!` macro.
//!
//! #### Limitations
//! - Annotated traits must have a `'static` bound (for now).
//! - Methods with non-lifetime generics are not supported.

use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use crate::prelude::StableAny;

mod any;
mod stable_any;

pub mod prelude {
    pub use thin_trait_objects_macros::thin;
    pub use crate::{
        Thin, //ThinRef, //ThinMut,
        ThinExt,
        RefSelf, MutSelf,
        Own, Ref, Mut, SpecialAssoc
    };

    pub use thin_trait_objects_macros::{
        StableAny, impl_stable_any
    };

    pub use crate::stable_any::{
        UUID, StableAny, StableTypeId
    };
}

#[repr(transparent)]
pub struct Thin<T: ?Sized + SpecialAssoc> {
    // type-erased `*mut Bundle<K> where `K: F` and `T` is `dyn F`
    pub ptr: NonNull<()>,
    phantom: PhantomData<T>,
}

unsafe impl<T: ?Sized + SpecialAssoc + Send> Send for Thin<T> {}
unsafe impl<T: ?Sized + SpecialAssoc + Sync> Sync for Thin<T> {}

impl<T: ?Sized + SpecialAssoc + 'static> Thin<T> {
    #[doc(hidden)]
    pub unsafe fn from_raw(ptr: *mut ()) -> Thin<T> {
        Thin {
            ptr: NonNull::new(ptr).unwrap(),
            phantom: PhantomData
        }
    }
}

pub trait ThinExt<U: ?Sized + SpecialAssoc +'static, T> {
    /// Creates a new `Thin<dyn _>` from the given value.
    fn new(val: T) -> Thin<U>;
}

//========================//
// impls to avoid double-indirection
// `&Thin<_>` or `&mut Thin<_>`


impl<T: ?Sized + SpecialAssoc + 'static> Thin<T> {
    fn as_ref(&self) -> Thin<&T> {
        Thin {
            ptr: self.ptr,
            phantom: PhantomData
        }
    }

    fn as_mut(&mut self) -> Thin<&mut T> {
        Thin {
            ptr: self.ptr,
            phantom: PhantomData
        }
    }
}

impl<T: ?Sized + SpecialAssoc + 'static> Thin<&T> {
    fn copy(&self) -> Thin<&T> {
        Thin {
            ptr: self.ptr,
            phantom: PhantomData
        }
    }
}

impl<T: ?Sized + SpecialAssoc + 'static> Deref for Thin<&T> {
    type Target = Thin<T>;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self as *const Thin<&T> as *const Thin<T>) }
    }
}

impl<T: ?Sized + SpecialAssoc + 'static> Deref for Thin<&mut T> {
    type Target = Thin<T>;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self as *const Thin<&mut T> as *const Thin<T>) }
    }
}

impl<T: ?Sized + SpecialAssoc + 'static> DerefMut for Thin<&mut T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self as *mut Thin<&mut T> as *mut Thin<T>) }
    }
}

//========================//
// dodgy hack for specialising the `Drop` impls of
// `Thin<T>`, `Thin<&T>`, and `Thin<&mut T>`

pub struct Own;
pub struct Ref;
pub struct Mut;

pub trait SpecialAssoc: SpecialParam<Self::Kind> {
    type Kind;
}

pub trait SpecialParam<K> {
    fn drop(ptr: NonNull<()>);
}

// T

/*
// this must be manually implemented for each concrete type
impl<T: ?Sized + 'static> SpecialAssoc for T {
    type Kind = Own;
}
*/

impl<T: ?Sized + SpecialAssoc + 'static> SpecialParam<Own> for T {
    fn drop(ptr: NonNull<()>) {
        // SAFETY: `Bundle` and `VTable` are `#[repr(C)]`,
        // so the `drop` field of `VTable` will be positioned first in the memory layout of `Bundle`.
        let dropper: extern "C" fn(*mut ()) = unsafe { *ptr.as_ptr().cast() };
        dropper(ptr.as_ptr());
    }
}

// &T

impl<T: ?Sized + SpecialAssoc + 'static> SpecialAssoc for &T {
    type Kind = Ref;
}
impl<T: ?Sized + SpecialAssoc + 'static> SpecialParam<Ref> for &T {
    fn drop(ptr: NonNull<()>) {
        // we dont own the pointed-to value
    }
}

// &mut T

impl<T: ?Sized + SpecialAssoc + 'static> SpecialAssoc for &mut T {
    type Kind = Mut;
}
impl<T: ?Sized + SpecialAssoc + 'static> SpecialParam<Mut> for &mut T {
    fn drop(ptr: NonNull<()>) {
        // we dont own the pointed-to value
    }
}

// Drop

impl<T: ?Sized + SpecialAssoc> Drop for Thin<T> {
    fn drop(&mut self) {
        <T as SpecialParam<<T as SpecialAssoc>::Kind>>::drop(self.ptr);
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
    pub fn new<T: ?Sized + SpecialAssoc>(thin: &'a Thin<T>) -> Self {
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
    pub fn new<T: ?Sized + SpecialAssoc>(thin: &'a mut Thin<T>) -> MutSelf<'a> {
        MutSelf {
            ptr: thin.ptr.as_ptr(),
            marker: PhantomData,
        }
    }
}

//========================//

#[cfg(test)]
mod tests {
    use crate::prelude::*;

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
    fn thin_foo() {
        let mut thin = Thin::<dyn Foo>::new(8u8);
        thin.add(1u8);
        assert_eq!(*thin.get(), 9u8);
    }

    #[thin]
    trait Maximal: 'static {
        fn ref_self(&self);
        fn mut_self(&mut self);
        #[allow(clippy::needless_lifetimes)]
        fn self_lifetime<'a>(&'a self) -> &'a u8;
        fn elided_self_lifetime<'a>(&self, other: &'a u8) -> &'a u8;
        fn more_lifetimes<'a, 'b>(&'a self, other: &'b u8) -> &'b u8;
    }

    #[test]
    fn borrowing() {
        let mut owned = Thin::<dyn Foo>::new(8u8);

        let mut borrow_mut = owned.as_mut();
        borrow_mut.add(1);
        drop(borrow_mut);

        let borrow = owned.as_ref();
        let clone = borrow.copy();

        let a = clone.get();
        assert_eq!(*a, 9u8);

        let b = borrow.get();
        assert_eq!(*b, 9u8);
    }
}

/// Example output of the `#[thin]` attribute
mod example_macro_expansion {
    use crate::prelude::*;

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
        struct Bundle<T> {
            vtable: VTable,
            value: T,
        }
        impl SpecialAssoc for dyn Foo { type Kind = Own; }
        impl<K: Foo> ThinExt<dyn Foo, K> for Thin<dyn Foo> {
            fn new(value: K) -> Self {
                let vtable = VTable { drop: drop::<K>, add: add::<K>, get: get::<K> };
                let bundle = Bundle { vtable, value };
                let ptr = Box::into_raw(Box::new(bundle));
                unsafe { Thin::from_raw(ptr as *mut ()) }
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