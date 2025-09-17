use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{parse_macro_input, parse_quote, FnArg, Ident, ItemTrait, Pat, TraitItemFn, Type, TypeReference};
use quote::quote;

// helper for parsing a functions in a trait definition
fn process_trait_item_fn(trait_name: Ident, item_fn: TraitItemFn) -> (Ident, TokenStream2, TokenStream2, TokenStream2) {
    let fn_name = item_fn.sig.ident;
    let return_type = item_fn.sig.output;

    let args = item_fn.sig.inputs.into_iter().collect::<Vec<_>>();

    let receiver = args.get(0).expect(format!("function {}::{} must have a receiver", trait_name, fn_name).as_str());


    let non_self_args = args.iter().skip(1);
    let mut arg_names = Vec::new();
    let mut arg_types = Vec::<Type>::new();
    let mut arg_convs = Vec::new();
    for arg in non_self_args {
        let FnArg::Typed(pat_type) = arg else {
            panic!("error parsing argument of `{}::{}`", trait_name, fn_name);
        };

        let arg_name = match &*pat_type.pat {
            Pat::Ident(pat_ident) => pat_ident.ident.clone(),
            _ => panic!("error parsing argument of `{}::{}`", trait_name, fn_name),
        };
        arg_names.push(arg_name.clone());

        // convert any reference types to pointers
        // as fn pointers can't have lifetime generics
        let arg_type = &*pat_type.ty;
        match arg_type {
            Type::Reference(
                TypeReference { elem, mutability, .. }
            ) => {
                match mutability {
                    None => {
                        arg_types.push(parse_quote! { *const #elem });
                        arg_convs.push(
                            quote! { let #arg_name = unsafe { &*#arg_name }; }
                        );
                    },
                    Some(_) => {
                        arg_types.push(parse_quote! { *mut #elem });
                        arg_convs.push(
                            quote!{ let #arg_name = unsafe { &mut *#arg_name }; }
                        );
                    },
                }
            }
            _ => {},
        };
    }

    let wrapper = quote! {
        extern "C" fn #fn_name<T: #trait_name>(ptr: *mut (), #(#arg_names: #arg_types),*) #return_type {
            let bundle = unsafe { &mut *(ptr as *mut Bundle<T>) };
            #(#arg_convs)*
            T::#fn_name(&mut bundle.value, #(#arg_names),*)
        }
    };

    let vtable_field = quote! {
        #fn_name: extern "C" fn(*mut (), #(#arg_types),*) #return_type
    };

    let trait_fn_impl = quote! {
        fn #fn_name(#(#args),*) #return_type {
            let vtable = unsafe { &*(self.ptr as *const VTable) };
            (vtable.#fn_name)(self.ptr, #(#arg_names),*)
        }
    };

    (fn_name, vtable_field, wrapper, trait_fn_impl)
}

#[proc_macro_attribute]
pub fn thin(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_trait = parse_macro_input!(item as ItemTrait);
    let trait_name = item_trait.ident.clone();

    let mut trait_item_fns = Vec::new();
    for trait_item in &item_trait.items {
        use syn::TraitItem::*;
        match trait_item {
            Fn(function) => {
                trait_item_fns.push(function.clone());
            }
            _ => panic!("non-function items within the trait are not currently supported."),
        }
    }

    let mut fn_names = Vec::new();
    let mut vtable_fields = Vec::new();
    let mut wrappers = Vec::new();
    let mut trait_fn_impls = Vec::new();
    for item_fn in trait_item_fns {
        let (fn_name, vtable_field, wrapper, trait_fn_impl)= process_trait_item_fn(trait_name.clone(), item_fn.clone());
        fn_names.push(fn_name);
        vtable_fields.push(vtable_field);
        wrappers.push(wrapper);
        trait_fn_impls.push(trait_fn_impl);
    }

    quote! {
        #item_trait



        const _: () = {
            #[repr(C)]
            struct VTable {
                // drop MUST be the first field.
                // see the `Drop` impl of `Thin`
                drop: extern "C" fn(*mut ()),
                #(#vtable_fields),*
            }

            #[repr(C)]
            struct Bundle<T: #trait_name> {
                // vtable MUST be the first field.
                // see the `Drop` impl of `Thin`
                vtable: VTable,
                value: T,
            }

            extern "C" fn drop<T: #trait_name>(ptr: *mut ()) {
                let bundle = ptr as *mut Bundle<T>;
                // SAFETY: `ptr` was created with `Box::into_raw`
                let _ = unsafe { Box::from_raw(bundle) };
            }

            #(#wrappers)*

            impl<T: #trait_name> ThinExt<dyn #trait_name, T> for Thin<dyn #trait_name> {
                fn new(value: T) -> Thin<dyn #trait_name> {
                    let vtable = VTable {
                        drop: drop::<T>,
                        #(#fn_names: #fn_names::<T>,)*
                    };

                    let bundle = Bundle {
                        vtable,
                        value,
                    };

                    let ptr = Box::into_raw(Box::new(bundle)) as *mut ();

                    unsafe { Thin::from_raw(ptr) }
                }
            }

            impl #trait_name for Thin<dyn #trait_name> {
                #(#trait_fn_impls)*
            }
        };
    }.into()
}