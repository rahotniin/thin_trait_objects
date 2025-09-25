use std::any::Any;
use crate::prelude::*;

mod provided;

pub unsafe trait UUID {
    const UUID: u64;
    fn uuid(&self) -> u64 {
        Self::UUID
    }
}

macro_rules! impl_thin {
    ($trait: ty) => {
        const _: () = {
            #[repr(C)]
            struct VTable {
                drop: extern "C" fn(*mut ()),
                uuid: u64,
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
                    let vtable = VTable { drop: drop::<K>, uuid: K::UUID };
                    let bundle = Bundle { vtable, value };
                    let ptr = Box::into_raw(Box::new(bundle));
                    unsafe { Thin::from_raw(ptr as *mut ()) }
                }
            }

            unsafe impl UUID for Thin<$trait> {
                const UUID: u64 = 0;

                fn uuid(&self) -> u64 {
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

                pub fn is<T: UUID>(&self) -> bool {
                    T::UUID == self.uuid()
                }

                pub fn downcast<T: UUID>(self) -> Option<T> {
                    if self.is::<T>() {
                        let val: T = unsafe { self.downcast_unchecked::<T>() };
                        return Some(val);
                    }
                    None
                }

                pub fn downcast_ref<T: UUID>(&self) -> Option<&T> {
                    if self.is::<T>() {
                        let val: &T = unsafe { self.downcast_ref_unchecked::<T>() };
                        return Some(val);
                    }
                    None
                }

                pub fn downcast_mut<T: UUID>(&mut self) -> Option<&mut T> {
                    if self.is::<T>() {
                        let val: &mut T = unsafe { self.downcast_mut_unchecked::<T>() };
                        return Some(val);
                    }
                    None
                }
            }
        };
    };
}

impl_thin!(dyn Any);
impl_thin!(dyn Any + Send);
impl_thin!(dyn Any + Send + Sync);

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use std::any::Any;
    use std::marker::PhantomData;
    use std::mem::ManuallyDrop;

    use crate::prelude::*;


    #[derive(UUID)]
    struct TestStruct<'a, T> {
        f: T,
        phantom: PhantomData<&'a ()>,
    }

    #[test]
    fn compilation_independence() {
        assert_eq!(TestStruct::<u8>::UUID, 5540856323980585692);
    }

    #[test]
    fn type_generics() {
        assert_ne!(TestStruct::<u8>::UUID, TestStruct::<u16>::UUID);
    }

    #[derive(UUID)]
    enum TestEnum<T> {
        Foo(T),
        Bar(TestStruct<'static, T>),
    }

    #[derive(UUID)]
    union TestUnion<A, B> {
        foo: ManuallyDrop<A>,
        bar: ManuallyDrop<B>,
    }

    #[test]
    fn downcasting() {
        let mut thin = Thin::<dyn Any>::new(8u8);

        let val = thin.downcast_ref::<u8>().unwrap();
        assert_eq!(*val, 8u8);

        let val = thin.downcast_mut::<u8>().unwrap();
        *val += 1;

        let val = thin.downcast::<u8>().unwrap();
        assert_eq!(val, 9u8);
    }
}