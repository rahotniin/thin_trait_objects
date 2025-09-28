use std::any::{Any, TypeId};
use crate::prelude::*;

macro_rules! impl_thin_dyn_any {
    ($($bounds: path),*) => {
        const _: () = {
            #[repr(C)]
            struct VTable {
                drop: extern "C" fn(*mut ()),
                type_id: TypeId,
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

            impl<K: Any $(+ $bounds)*> ThinExt<dyn Any $(+ $bounds)*, K> for Thin<dyn Any $(+ $bounds)*> {
                fn new(value: K) -> Self {
                    let vtable = VTable { drop: drop::<K>, type_id: TypeId::of::<K>() };
                    let bundle = Bundle { vtable, value };
                    let ptr = Box::into_raw(Box::new(bundle));
                    unsafe { Thin::from_raw(ptr as *mut ()) }
                }
            }

            impl SpecialAssoc for dyn Any $(+ $bounds)* {
                type Kind = Own;
            }

            impl Thin<dyn Any $(+ $bounds)*> {
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

                pub fn is<T: 'static>(&self) -> bool {
                    let vtable = unsafe { &*(self.ptr.as_ptr() as *const VTable) };
                    vtable.type_id == TypeId::of::<T>()
                }

                pub fn downcast<T: 'static>(self) -> Option<T> {
                    if self.is::<T>() {
                        let val: T = unsafe { self.downcast_unchecked::<T>() };
                        return Some(val);
                    }
                    None
                }

                pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
                    if self.is::<T>() {
                        let val: &T = unsafe { self.downcast_ref_unchecked::<T>() };
                        return Some(val);
                    }
                    None
                }

                pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
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

impl_thin_dyn_any!();
impl_thin_dyn_any!(Send);
impl_thin_dyn_any!(Send, Sync);

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use std::any::Any;

    use crate::prelude::*;

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