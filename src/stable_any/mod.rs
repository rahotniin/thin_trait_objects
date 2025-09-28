use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use crate::prelude::*;
use crate::{Own, SpecialAssoc};

/// Module providing implementations of `UUID` for various foreign types.
mod provided;

pub unsafe trait UUID {
    const UUID: StableTypeId;
}

#[derive(PartialEq, Eq, Copy, Clone, Hash)]
pub struct StableTypeId(u64);

impl StableTypeId {
    const unsafe fn new(val: u64) -> Self {
        Self(val)
    }

    const unsafe fn to_u64(self) -> u64 {
        self.0
    }

    pub const fn of<T: StableAny>() -> StableTypeId {
        T::Inner::UUID
    }
}

impl Debug for StableTypeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

mod private {
    pub trait Sealed {}
}

pub trait StableAny: private::Sealed {
    #[doc(hidden)]
    type Inner: UUID + ?Sized where Self: Sized;
    fn stable_type_id(&self) -> StableTypeId;
}

impl<T: UUID> private::Sealed for T {}
impl<T: UUID> StableAny for T {
    type Inner = T;
    fn stable_type_id(&self) -> StableTypeId {
        T::UUID
    }
}

//================//

macro_rules! impl_thin_dyn_stable_any {
    ($($bounds: path),*) => {
        const _: () = {
            #[repr(C)]
            struct VTable {
                drop: extern "C" fn(*mut ()),
                uuid: StableTypeId,
            }

            extern "C" fn drop<T>(ptr: *mut ()) {
                let bundle = ptr as *mut Bundle<T>;
                let _ = unsafe { Box::from_raw(bundle) };
            }

            #[repr(C)]
            struct Bundle<T> {
                vtable: VTable,
                value: T,
            }

            impl<K: StableAny $(+ $bounds)*> ThinExt<dyn StableAny $(+ $bounds)*, K> for Thin<dyn StableAny $(+ $bounds)*> {
                fn new(value: K) -> Self {
                    let vtable = VTable { drop: drop::<K>, uuid: StableTypeId::of::<K>() };
                    let bundle = Bundle { vtable, value };
                    let ptr = Box::into_raw(Box::new(bundle));
                    unsafe { Thin::from_raw(ptr as *mut ()) }
                }
            }

            impl SpecialAssoc for dyn StableAny $(+ $bounds)* {
                type Kind = Own;
            }

            impl private::Sealed for Thin<dyn StableAny $(+ $bounds)*> {}
            impl StableAny for Thin<dyn StableAny $(+ $bounds)*> {
                type Inner = dyn StableAny $(+ $bounds)*;
                fn stable_type_id(&self) -> StableTypeId {
                    let vtable = unsafe { &*(self.ptr.as_ptr() as *const VTable) };
                    vtable.uuid
                }
            }

            impl Thin<dyn StableAny $(+ $bounds)*> {
                unsafe fn downcast_unchecked<T>(self) -> T {
                    let ptr = self.ptr.as_ptr() as *mut Bundle<T>;
                    ::std::mem::forget(self);
                    let bundle = unsafe { Box::from_raw(ptr) };
                    bundle.value
                }

                unsafe fn downcast_ref_unchecked<T>(&self) -> &T {
                    let ptr = self.ptr.as_ptr() as *const Bundle<T>;
                    let bundle = unsafe { &*ptr };
                    &bundle.value
                }

                unsafe fn downcast_mut_unchecked<T>(&mut self) -> &mut T {
                    let ptr = self.ptr.as_ptr() as *mut Bundle<T>;
                    let bundle = unsafe { &mut *ptr };
                    &mut bundle.value
                }

                pub fn stable_is<T: UUID>(&self) -> bool {
                    T::UUID == StableAny::stable_type_id(self)
                }

                pub fn downcast<T: UUID>(self) -> Option<T> {
                    if self.stable_is::<T>() {
                        let val: T = unsafe { self.downcast_unchecked::<T>() };
                        return Some(val);
                    }
                    None
                }

                pub fn downcast_ref<T: UUID>(&self) -> Option<&T> {
                    if self.stable_is::<T>() {
                        let val: &T = unsafe { self.downcast_ref_unchecked::<T>() };
                        return Some(val);
                    }
                    None
                }

                pub fn downcast_mut<T: UUID>(&mut self) -> Option<&mut T> {
                    if self.stable_is::<T>() {
                        let val: &mut T = unsafe { self.downcast_mut_unchecked::<T>() };
                        return Some(val);
                    }
                    None
                }
            }
        };
    };
}

impl_thin_dyn_stable_any!();
impl_thin_dyn_stable_any!(Send);
impl_thin_dyn_stable_any!(Send, Sync);

// the following UUIDs where randomly generated using
// https://numbergenerator.org/random-16-digit-hex-codes-generator

unsafe impl UUID for dyn StableAny {
    const UUID: StableTypeId = StableTypeId(0xB3B4E57FBD6B8818u64);
}

unsafe impl UUID for dyn StableAny + Send {
    const UUID: StableTypeId = StableTypeId(0x5B16BCA86ABA3C25u64);
}

unsafe impl UUID for dyn StableAny + Send + Sync {
    const UUID: StableTypeId = StableTypeId(0xFDB2A76E12E2D8D8u64);
}

impl SpecialAssoc for Thin<dyn StableAny> {
    type Kind = Own;
}

//================//

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use std::fmt::Display;
    use std::marker::PhantomData;
    use std::mem::ManuallyDrop;

    use crate::prelude::*;
    use crate::stable_any::StableAny;

    #[derive(StableAny)]
    struct TestStruct<'a, T> {
        f: T,
        phantom: PhantomData<&'a ()>,
    }

    #[test]
    fn compilation_independence() {
        // this will fail every time the crates version changes
        // or the path of this module changes
        // TODO: stop using the patch version in generating UUIDs
        assert_eq!(
            StableTypeId::of::<TestStruct<u8>>(),
            unsafe { StableTypeId::new(109550671095340697) }
        );
    }

    #[test]
    fn thin_uuids() {
        assert_eq!(
            StableTypeId::of::<Thin<dyn StableAny>>(),
            unsafe { StableTypeId::new(12949227165398566936) }
        );

        assert_eq!(
            StableTypeId::of::<Thin<dyn StableAny + Send>>(),
            unsafe { StableTypeId::new(6563640938470194213) }
        );

        assert_eq!(
            StableTypeId::of::<Thin<dyn StableAny + Send + Sync>>(),
            unsafe { StableTypeId::new(18280857928655362264) }
        );
    }

    #[test]
    fn type_generics() {
        assert_ne!(StableTypeId::of::<TestStruct<u8>>(), StableTypeId::of::<TestStruct<u16>>());
    }

    #[derive(StableAny)]
    enum TestEnum<T: Display> {
        Foo(T),
        Bar(TestStruct<'static, T>),
    }

    #[derive(StableAny)]
    union TestUnion<A, B> {
        foo: ManuallyDrop<A>,
        bar: ManuallyDrop<B>,
    }

    #[test]
    fn downcasting() {
        let mut thin = Thin::<dyn StableAny>::new(8u8);

        let val = thin.downcast_ref::<u8>().unwrap();
        assert_eq!(*val, 8u8);

        let val = thin.downcast_mut::<u8>().unwrap();
        *val += 1;

        let val = thin.downcast::<u8>().unwrap();
        assert_eq!(val, 9u8);
    }
}