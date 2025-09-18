use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, parse_quote, AngleBracketedGenericArguments, FnArg, GenericArgument, Generics, Ident, ItemTrait, Pat, PatIdent, Path, PathArguments, PathSegment, ReturnType, TraitItem, Type, TypeParamBound, TypePath, TypeReference, TypeTuple};

// TODO: slim this boy down with some helper functions
#[proc_macro_attribute]
pub fn thin(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item_trait = parse_macro_input!(item as ItemTrait);
    let trait_name = &item_trait.ident;

    let super_traits = &mut item_trait.supertraits;

    let mut is_static = false;
    let static_bound: TypeParamBound = parse_quote!('static);
    for super_trait in super_traits.iter() {
        if *super_trait == static_bound { is_static = true; break }
    }
    if !is_static {
        panic!("Error parsing {}: Traits without a `'static` bound are currently not supported", trait_name);
    }

    let trait_items = &item_trait.items.clone();

    let mut fn_names = Vec::new();
    let mut vtable_fields = Vec::new();
    let mut shims = Vec::new();
    let mut trait_method_impls = Vec::new();

    for item in trait_items {
        let TraitItem::Fn(function) = item else {
            panic!("non-function items are not supported");
        };

        let fn_name = &function.sig.ident;
        fn_names.push(fn_name.clone());

        let generics = &function.sig.generics;
        forbid_non_lifetime_generics(generics, trait_name, fn_name);

        let args = function.sig.inputs.iter().collect::<Vec<_>>();
        let mut arg_names = Vec::new();
        let mut arg_types = Vec::new();

        //================//
        // receiver

        let Some(FnArg::Receiver(recv)) = args.get(0) else {
            // the compiler should catch misplaced receivers before we get here
            // so I reckon this is unnecessary
            panic!("{}::{} must have a receiver", trait_name, fn_name);
        };

        let lt = match recv.lifetime() {
            Some(lt) => lt.clone(),
            None => parse_quote!('_),
        };

        let recv_type: Type;
        let erase_recv: TokenStream2;
        let un_erase_recv: TokenStream2;
        match recv.mutability {
            None => {
                recv_type = parse_quote!(RefSelf<#lt>);
                erase_recv = quote! {
                    let recv = RefSelf::new(self);
                };
                un_erase_recv = quote! {
                    let bundle = unsafe { &*(recv.ptr as *const Bundle<T>) };
                    let recv = &bundle.value;
                };
            },
            Some(_) => {
                recv_type = parse_quote!(MutSelf<#lt>);
                erase_recv = quote! {
                    let recv = MutSelf::new(self);
                };
                un_erase_recv = quote! {
                    let bundle = unsafe { &mut *(recv.ptr as *mut Bundle<T>) };
                    let recv = &mut bundle.value;
                };
            },
        }

        arg_names.push(parse_quote!(recv));
        arg_types.push(recv_type);

        //================//
        // non-receiver arguments

        for arg in &args[1..] {
            let FnArg::Typed(pat_type) = arg else {
                // SAFETY: the earlier let-else match on the receiver should ensure we never get here
                unsafe { std::hint::unreachable_unchecked() };
            };

            let arg_name = match &*pat_type.pat {
                Pat::Ident(PatIdent { ident: name, .. }) => name,
                _ => panic!("Error parsing argument of {}::{}", trait_name, fn_name),
            };

            arg_names.push(arg_name.clone());

            let mut arg_type = *pat_type.ty.clone();
            if let Err(ty) = un_elide_lifetimes(&mut arg_type) {
                panic!("Error parsing `{}::{}`: Arguments of type `{}` not supported", trait_name, fn_name, quote!(#ty));
            }

            arg_types.push(arg_type);
        }

        //================//
        // return type

        let mut return_type = function.sig.output.clone();
        match &mut return_type {
            ReturnType::Type(_, ty) => {
                if let Err(ty) = un_elide_lifetimes(ty) {
                    panic!("Error parsing `{}::{}`: `{}` is not supported in return types", trait_name, fn_name, quote!(#ty));
                }
            }
            _ => {}
        }

        //================//
        // for clause

        let mut lifetimes = generics.lifetimes();
        let for_clause = match lifetimes.next() {
            None => None,
            Some(first) => Some(quote! { for<#first #(, #lifetimes)*> }),
        };
        
        //================//
        // putting it all together

        let vtable_field = quote! {
            #fn_name: #for_clause extern "C" fn (#(#arg_types),*) #return_type,
        };

        let shim = quote! {
            extern "C" fn #fn_name<T: #trait_name> (#(#arg_names: #arg_types),*) #return_type {
                // no references to the vtable should exist at this point
                #un_erase_recv
                T::#fn_name(#(#arg_names),*)
            }
        };

        let trait_method_impl = quote! {
            fn #fn_name(#(#args),*) #return_type {
                let shim = {
                    // SAFETY:
                    // see https://adventures.michaelfbryan.com/posts/ffi-safe-polymorphism-in-rust/?utm_source=user-forums&utm_medium=social&utm_campaign=thin-trait-objects#pointer-to-vtable--object
                    let vtable = unsafe { &*(self.ptr.as_ptr() as *const VTable) };
                    vtable.#fn_name
                    // reference to vtable dropped here?
                };
                #erase_recv
                shim(#(#arg_names),*)
            }
        };

        vtable_fields.push(vtable_field);
        shims.push(shim);
        trait_method_impls.push(trait_method_impl);
    }

    quote! {
        #item_trait

        const _: () = {
            #[repr(C)]
            struct VTable {
                drop: extern "C" fn(*mut ()),
                #(#vtable_fields)*
            }

            extern "C" fn drop<T: #trait_name>(ptr: *mut ()) {
                let bundle = ptr as *mut Bundle<T>;
                let _ = unsafe { Box::from_raw(bundle) };
            }

            #(#shims)*

            #[repr(C)]
            struct Bundle<T: #trait_name> {
                vtable: VTable,
                value: T
            }

            impl<T: #trait_name> ThinExt<dyn #trait_name, T> for Thin<dyn #trait_name> {
                fn new(value: T) -> Self {
                    let vtable = VTable {
                        drop: drop::<T>,
                        #(#fn_names: #fn_names::<T>),*
                    };

                    let bundle = Bundle {
                        vtable,
                        value,
                    };

                    let ptr = Box::into_raw(Box::new(bundle));

                    unsafe { Thin::from_raw(ptr as *mut ()) }
                }

                unsafe fn downcast_unchecked(self) -> T {
                    let ptr = self.ptr.as_ptr() as *mut Bundle<T>;
                    ::std::mem::forget(self);
                    let bundle = unsafe { Box::from_raw(ptr) };
                    bundle.value
                }
            }

            impl #trait_name for Thin<dyn #trait_name> {
                #(#trait_method_impls)*
            }
        };
    }.into()
}

/// Un-elides a `Types`s lifetimes by inserting `'_` where explicit lifetimes would otherwise be.
fn un_elide_lifetimes(ty: &mut Type) -> Result<(), Type> {
    // TODO: support for more types
    match ty {
        Type::Reference(TypeReference { lifetime, .. }) => {
            match lifetime {
                None => *lifetime = Some(parse_quote!('_)),
                _ => {}
            }
        }
        Type::Tuple(TypeTuple { elems, .. }) => {
            for elem in elems {
                un_elide_lifetimes(elem)?
            }
        }
        Type::Path(TypePath { path: Path { segments, ..}, .. }) => {
            for segment in segments {
                let PathSegment { arguments, .. } = segment;
                match arguments {
                    PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, ..}) => {
                        for arg in args {
                            match arg {
                                GenericArgument::Type(ty) => un_elide_lifetimes(ty)?,
                                _ => {}
                            }
                        }
                    }
                    PathArguments::None => {}
                    _ => return Err(ty.clone()),
                }
            }
        }
        _ => return Err(ty.clone()),
    };

    Ok(())
}

fn forbid_non_lifetime_generics(generics: &Generics, trait_name: &Ident, fn_name: &Ident) {
    let type_generics = generics.type_params();
    for _ in type_generics {
        panic!("Error parsing `{}::{}`: type generics are not supported", trait_name, fn_name);
    }

    let const_generics = generics.const_params();
    for _ in const_generics {
        panic!("Error parsing `{}::{}`: const generics are not supported", trait_name, fn_name);
    }
}