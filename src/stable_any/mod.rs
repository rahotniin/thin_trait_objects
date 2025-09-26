use std::fmt::{Debug, Formatter};
use crate::prelude::*;

/// Module providing implementations of `UUID` for various foreign types.
mod provided;

pub unsafe trait UUID {
    const UUID: StableTypeId;
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub struct StableTypeId(u64);

impl StableTypeId {
    const unsafe fn new(val: u64) -> Self {
        Self(val)
    }

    const unsafe fn to_u64(self) -> u64 {
        self.0
    }

    pub const fn of<T: private::StableAny>() -> StableTypeId {
        T::Inner::UUID
    }
}

impl Debug for StableTypeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

mod private {
    use super::*;

    pub unsafe trait StableAny {
        type Inner: UUID + ?Sized;
        fn stable_type_id(&self) -> StableTypeId;
    }

    unsafe impl<T: UUID> StableAny for T {
        type Inner = T;
        fn stable_type_id(&self) -> StableTypeId {
            T::UUID
        }
    }
}

pub trait StableAny: private::StableAny<Inner = ()> {
    fn stable_type_id(&self) -> StableTypeId {
        <<Self as private::StableAny>::Inner as UUID>::UUID
    }
}

macro_rules! impl_thin {
    ($trait: ty) => {
        const _: () = {
            #[repr(C)]
            struct VTable {
                drop: extern "C" fn(*mut ()),
                uuid: StableTypeId,
            }

            extern "C" fn drop<T: UUID>(ptr: *mut ()) {
                let bundle = ptr as *mut Bundle<T>;
                let _ = unsafe { Box::from_raw(bundle) };
            }

            #[repr(C)]
            struct Bundle<T> {
                vtable: VTable,
                value: T,
            }

            impl<K: UUID> ThinExt<$trait, K> for Thin<$trait> {
                fn new(value: K) -> Self {
                    let vtable = VTable { drop: drop::<K>, uuid: StableTypeId::of::<K>() };
                    let bundle = Bundle { vtable, value };
                    let ptr = Box::into_raw(Box::new(bundle));
                    unsafe { Thin::from_raw(ptr as *mut ()) }
                }
            }

            unsafe impl private::StableAny for Thin<$trait> {
                type Inner = ();
                fn stable_type_id(&self) -> StableTypeId {
                    let vtable = unsafe { &*(self.ptr.as_ptr() as *const VTable) };
                    vtable.uuid
                }
            }

            impl Thin<$trait> {
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
                    T::UUID == private::StableAny::stable_type_id(self)
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

impl_thin!(dyn StableAny);
impl_thin!(dyn StableAny + Send);
impl_thin!(dyn StableAny + Send + Sync);

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
        assert_eq!(StableTypeId::of::<TestStruct<u8>>(), unsafe { StableTypeId::new(7578435656508451722) });
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