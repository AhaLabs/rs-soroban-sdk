use deluxe::HasAttributes;
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse_quote, punctuated::Punctuated, spanned::Spanned, Attribute, FnArg, Item, ItemImpl,
    ItemTrait, PatType, Signature, Token, TraitItem, TraitItemFn, TraitItemType, Type,
};

pub(crate) mod args;
mod util;

use args::{InnerArgs, MyMacroArgs, MyTraitMacroArgs};
use syn::Error;
use util::has_attr;

pub fn generate_trait(args: MyTraitMacroArgs, item: &ItemTrait) -> TokenStream {
    inner_generate(args, item).unwrap_or_else(|e| e.to_compile_error())
}

pub fn derive_contract(args: &MyMacroArgs, trait_impls: &Item) -> TokenStream {
    derive_contract_inner(args, trait_impls).unwrap_or_else(|e| e.to_compile_error())
}

fn generate_method(
    (trait_item, item_trait): (&syn::TraitItem, &syn::ItemTrait),
) -> Option<(Option<TokenStream>, TokenStream)> {
    let syn::TraitItem::Fn(mut method) = trait_item.clone() else {
        return None;
    };
    let sig = &method.sig;
    let name = &sig.ident;
    if sig.receiver().is_some() {
        return None;
    };
    let args = args_to_idents(&sig.inputs);
    let attrs = &method.attrs;
    if has_attr(attrs, "internal") {
        method
            .attrs
            .retain(|attr| !attr.path().is_ident("internal"));
        let method_stream = if method.default.is_none() {
            generate_trait_method(&method, name, &args)
        } else {
            method.to_token_stream()
        };
        return Some((None, method_stream));
    }
    if method.default.is_some() {
        return Some((
            Some(generate_static_method(item_trait, sig, attrs, name, &args)),
            method.to_token_stream(),
        ));
    }
    Some((
        Some(generate_static_method(item_trait, sig, attrs, name, &args)),
        generate_trait_method(&method, name, &args),
    ))
}

fn arg_to_ident(arg: &FnArg) -> Option<&Ident> {
    if let FnArg::Typed(PatType { pat, .. }) = arg {
        if let syn::Pat::Ident(pat_ident) = &**pat {
            return Some(&pat_ident.ident);
        }
    }
    None
}
pub fn args_to_idents(inputs: &Punctuated<FnArg, Token!(,)>) -> Vec<&Ident> {
    inputs.iter().filter_map(arg_to_ident).collect::<Vec<_>>()
}

fn generate_static_method(
    trait_name: &ItemTrait,
    sig: &Signature,
    attrs: &[Attribute],
    name: &Ident,
    args: &[&Ident],
) -> TokenStream {
    let trait_name = &trait_name.ident;
    let output = &sig.output;

    // Transform inputs and generate call arguments
    let (transformed_inputs, call_args): (Vec<_>, Vec<_>) = sig
        .inputs
        .iter()
        .zip(args.iter())
        .filter_map(|(input, arg_name)| {
            if let FnArg::Typed(PatType { pat, ty, .. }) = input {
                let (new_ty, call_expr) = transform_type_and_call(ty, arg_name);
                Some((quote! { #pat: #new_ty }, call_expr))
            } else {
                // Skip 'self' parameters
                None
            }
        })
        .unzip();

    quote! {
        #(#attrs)*
        pub fn #name(#(#transformed_inputs),*) #output {
            <$contract_name as #trait_name>::#name(#(#call_args),*)
        }
    }
}

fn transform_type_and_call(ty: &Type, arg_name: &Ident) -> (TokenStream, TokenStream) {
    match ty {
        // &T -> T, call with &arg
        Type::Reference(type_ref) if type_ref.mutability.is_none() => {
            let inner_type = &type_ref.elem;
            (quote! { #inner_type }, quote! { &#arg_name })
        }
        // &mut T -> T, call with &mut arg
        Type::Reference(type_ref) if type_ref.mutability.is_some() => {
            let inner_type = &type_ref.elem;
            (quote! { #inner_type }, quote! { &mut #arg_name })
        }
        // Any other type -> keep as is, call with arg
        _ => (quote! { #ty }, quote! { #arg_name }),
    }
}

fn generate_trait_method(method: &syn::TraitItemFn, name: &Ident, args: &[&Ident]) -> TokenStream {
    let mut method = method.clone();
    method.default = Some(syn::parse_quote! {
        {
            Self::Impl::#name(#(#args),*)
        }
    });
    method.to_token_stream()
}

fn generate_never_impl(
    trait_ident: &syn::Ident,
    never_ident: &syn::Ident,
    items: &[TraitItemFn],
) -> TokenStream {
    let methods = items.iter().map(
            |TraitItemFn {
                 attrs,
                 sig,
                 ..
             }| {
                 let message = format!(
            "The contract trait `{trait_ident}` requires an extension for authentication but none were provided.\
            E.g. #[derive_contract(Administratable, Upgradable(ext = AdministratableExt))]",
        );
                quote! {
                    #(#attrs)*
                    #sig {
                        compile_error!(#message);
                    }
                }

             },
        ).collect::<Vec<_>>();
    quote! {
        pub struct #never_ident<N>(
            core::marker::PhantomData<N>,
        );
        impl<N:#trait_ident> #trait_ident for #never_ident<N> {
            type Impl = N;
            #(#methods)*
        }
    }
}

fn inner_generate(
    MyTraitMacroArgs {
        default,
        ext_required,
        is_ext,
    }: MyTraitMacroArgs,
    input_trait: &ItemTrait,
) -> Result<TokenStream, Error> {
    let (generated_methods, trait_methods): (Vec<_>, Vec<_>) = input_trait
        .items
        .iter()
        .zip(std::iter::repeat(input_trait))
        .filter_map(generate_method)
        .unzip();

    let trait_ident = &input_trait.ident;
    let macro_rules_name = trait_ident;
    let mut trait_ = input_trait.clone();
    let items = trait_methods
        .into_iter()
        .map(syn::parse2)
        .collect::<Result<Vec<TraitItemFn>, _>>()?;

    let never_ident = format_ident!("{}Never", trait_ident);
    let never_ext_impl =
        ext_required.then(|| generate_never_impl(trait_ident, &never_ident, &items));

    trait_.items = trait_
        .items
        .into_iter()
        .filter(|item| !matches!(item, TraitItem::Fn(_)))
        .chain(items.into_iter().map(TraitItem::Fn))
        .collect();
    if !trait_
        .items
        .iter()
        .any(|item| matches!(item, TraitItem::Type(TraitItemType { ident, .. }) if ident == "Impl"))
    {
        trait_.items.insert(
            0,
            syn::parse_quote! {
                type Impl: #trait_ident;
            },
        );
    }
    let default_impl = default
        .as_ref()
        .map_or_else(|| quote! {$contract_name}, Ident::to_token_stream);

    let default_used = if ext_required {
        quote! { #never_ident<$crate::#default_impl> }
    } else {
        quote! { $crate::#default_impl }
    };

    let ensure_default = default.is_none().then(|| {
        let message = format!(
            "The contract trait `{trait_ident}` does not provide default implementation. \
One should be passed, e.g. `#[derive_contract(Administratable(default = MyAdmin))"
        );
        quote! {
            compile_error!(#message);
        }
    });

    let extension_type = is_ext.then(|| {
        let extension_strukt = format_ident!("{trait_ident}Ext");
        quote! {
            pub struct #extension_strukt<T: #trait_ident, N>(
                  core::marker::PhantomData<T>,
                  core::marker::PhantomData<N>,
            );
        }
    });
    let docs = input_trait
        .attrs()
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .collect::<Vec<_>>();

    let output = quote! {

    #trait_
    #extension_type
    #(#docs)*
    #[macro_export]
    macro_rules! #macro_rules_name {
        ($contract_name:ident) => {
            #ensure_default
            #never_ext_impl
            #macro_rules_name!($contract_name, #default_used);
        };
        ($contract_name:ident, $impl_name:path) => {
            impl #trait_ident for $contract_name {
                type Impl = $impl_name;
            }
            #macro_rules_name!($contract_name, $contract_name, $contract_name);
        };
        ($contract_name:ident, $_a:ident, $_b:ident) => {
            #[soroban_sdk::contractimpl]
            impl $contract_name {
                #(#generated_methods)*
            }
        };
        () => {
            $crate::#default_impl
        };

            }
    };
    Ok(output)
}

pub fn derive_trait_impl_external(mut impl_: ItemImpl, args: &InnerArgs) -> TokenStream {
    let Some((_, trait_, _)) = impl_.trait_.as_ref() else {
        return Error::new(impl_.span(), "Input must be a impl with a trait")
            .into_compile_error()
            .into();
    };
    let type_path = match *impl_.self_ty.clone() {
        Type::Path(type_path) => type_path.path,
        _ => todo!(),
    };
    let strukt_ident = parse_quote!(#type_path);
    let (macro_calls, default) = derive_trait_impl(trait_, args, strukt_ident, true);
    if !impl_.items.iter().any(
        |item| matches!(item, syn::ImplItem::Type(syn::ImplItemType {ident, ..}) if ident == "Impl"),
    ) {
        impl_.items.insert(
            0,
          syn::parse_quote! {
            type Impl = #default;
          });
    }
    quote! {
        #impl_
        #macro_calls
    }
}

pub fn derive_trait_impl(
    trait_ident: &syn::Path,
    InnerArgs { exts, default }: &InnerArgs,
    strukt_type: syn::Ident,
    is_impl_: bool,
) -> (TokenStream, TokenStream) {
    if exts.is_empty() && default.is_none() && !is_impl_ {
        return (
            quote! { #trait_ident!(#strukt_type); },
            TokenStream::default(),
        );
    }
    let is_self = default
        .as_ref()
        .map(|i| {
            let ident: Ident = parse_quote!(#i);

            ident == strukt_type
        })
        .unwrap_or(false);
    let init = default
        .as_ref()
        .map_or_else(|| quote! {#trait_ident!()}, ToTokens::to_token_stream);
    let default_impl = exts.iter().fold(
        init,
        |acc, extension| quote! { #extension<#strukt_type, #acc> },
    );
    let call = if is_self || is_impl_ {
        quote! {
            #trait_ident!(#strukt_type, #strukt_type, #strukt_type);
        }
    } else {
        quote! {
            #trait_ident!(#strukt_type, #default_impl);
        }
    };
    (call, default_impl)
}

pub fn derive_contract_inner(args: &MyMacroArgs, trait_impls: &Item) -> Result<TokenStream, Error> {
    let Item::Struct(strukt) = trait_impls else {
        return Err(Error::new(trait_impls.span(), "Input must be a struct"));
    };
    let strukt_name = &strukt.ident;
    let macro_calls = args
        .args
        .iter()
        .sorted_by(|(a, _), (b, _)| {
            a.to_token_stream()
                .to_string()
                .cmp(&b.to_token_stream().to_string())
        })
        .map(|(trait_ident, args)| {
            derive_trait_impl(trait_ident, args, strukt_name.clone(), false).0
        });
    let output = quote! {
        #strukt
        #(#macro_calls)*
    };
    Ok(output)
}

#[cfg(test)]
mod tests {

    use super::util::*;
    use super::*;

    #[test]
    fn contracttrait_on_trait() {
        let input: ItemTrait = syn::parse_quote! {
            pub trait Administratable {
                /// Get current admin
                fn admin_get(env: Env) -> soroban_sdk::Address;
                fn admin_set(env: Env, new_admin: &soroban_sdk::Address);
                #[internal]
                fn require_auth(env: Env) {
                    Self::admin_get(env).require_auth();
                }
            }
        };
        let default = Some(format_ident!("Admin"));
        let actual: TokenStream = generate_trait(
            MyTraitMacroArgs {
                default,
                ..Default::default()
            },
            &input,
        );

        let expected = quote! {
        pub trait Administratable {
            type Impl: Administratable;
            #[doc = r" Get current admin"]
            fn admin_get(env: Env) -> soroban_sdk::Address {
                Self::Impl::admin_get(env)
            }
            fn admin_set(env: Env, new_admin: &soroban_sdk::Address) {
                Self::Impl::admin_set(env, new_admin)
            }
            fn require_auth(env: Env) {
                Self::admin_get(env).require_auth();
            }
        }
        #[macro_export]
        macro_rules! Administratable {
            ($contract_name: ident) => {
                Administratable!($contract_name, $crate::Admin);
            };

             ($contract_name: ident, $impl_name: path) => {
                impl Administratable for $contract_name {
                    type Impl = $impl_name;
                }
                Administratable!($contract_name, $contract_name, $contract_name);
             };
            ($contract_name: ident, $_a: ident, $_b: ident) => {
                #[soroban_sdk::contractimpl]
                impl $contract_name {
                    #[doc = r" Get current admin"]
                    pub fn admin_get(env: Env) -> soroban_sdk::Address {
                        < $contract_name as Administratable >::admin_get(env)
                    }

                    pub fn admin_set(env: Env, new_admin: soroban_sdk::Address) {
                        < $contract_name as Administratable >::admin_set(env, &new_admin)
                    }
                }
            };

            () => {
                $crate::Admin
            };
        }


                };
        equal_tokens(&expected, &actual);
    }
    #[test]
    fn contracttrait_on_trait_with_impl() {
        let input: ItemTrait = syn::parse_quote! {
            pub trait Administratable {
                type Impl: Administratable + Clone;
                /// Get current admin
                fn admin_get(env: Env) -> soroban_sdk::Address;
                fn admin_set(env: Env, new_admin: &soroban_sdk::Address);
                #[internal]
                fn require_auth(env: Env) {
                    Self::admin_get(env).require_auth();
                }
            }
        };
        let default = Some(format_ident!("Admin"));
        let actual: TokenStream = generate_trait(
            MyTraitMacroArgs {
                default,
                ..Default::default()
            },
            &input,
        );

        let expected = quote! {
        pub trait Administratable {
            type Impl: Administratable + Clone;
            #[doc = r" Get current admin"]
            fn admin_get(env: Env) -> soroban_sdk::Address {
                Self::Impl::admin_get(env)
            }
            fn admin_set(env: Env, new_admin: &soroban_sdk::Address) {
                Self::Impl::admin_set(env, new_admin)
            }
            fn require_auth(env: Env) {
                Self::admin_get(env).require_auth();
            }
        }
        #[macro_export]
        macro_rules! Administratable {
            ($contract_name: ident) => {
                Administratable!($contract_name, $crate::Admin);
            };

             ($contract_name: ident, $impl_name: path) => {
                impl Administratable for $contract_name {
                    type Impl = $impl_name;
                }
                Administratable!($contract_name, $contract_name, $contract_name);
             };
            ($contract_name: ident, $_a: ident, $_b: ident) => {
                #[soroban_sdk::contractimpl]
                impl $contract_name {
                    #[doc = r" Get current admin"]
                    pub fn admin_get(env: Env) -> soroban_sdk::Address {
                        < $contract_name as Administratable >::admin_get(env)
                    }

                    pub fn admin_set(env: Env, new_admin: soroban_sdk::Address) {
                        < $contract_name as Administratable >::admin_set(env, &new_admin)
                    }
                }
            };

            () => {
                $crate::Admin
            };
        }


                };
        equal_tokens(&expected, &actual);
    }

    #[test]
    fn derive() {
        let input: Item = syn::parse_quote! {
            pub struct Contract;
        };
        let args = vec![
            (
                syn::parse_quote! {Administratable},
                InnerArgs {
                    exts: vec![],
                    default: None,
                },
            ),
            (
                syn::parse_quote!(Upgradable),
                InnerArgs {
                    exts: vec![syn::parse_quote!(AdministratableExt)],
                    default: None,
                },
            ),
        ];

        let result = derive_contract(
            &MyMacroArgs {
                args: args.into_iter().collect(),
            },
            &input,
        );
        let output = quote! {
        pub struct Contract;
        Administratable!(Contract);
        Upgradable ! (Contract , AdministratableExt < Contract , Upgradable ! () >);
        };
        equal_tokens(&output, &result);
    }

    #[test]
    fn derive_on_impl() {
        let input = syn::parse_quote! {
            impl Administratable for Contract {}
        };
        let args = InnerArgs::default();

        let result = derive_trait_impl_external(input, &args);
        let output = quote! {
        impl Administratable for Contract {
            type Impl = Administratable!();
        }
        Administratable!(Contract, Contract, Contract);
        };
        equal_tokens(&output, &result);
    }
}
