use deluxe::HasAttributes;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    punctuated::Punctuated, spanned::Spanned, Attribute, FnArg, Item, ItemTrait, PatType,
    Signature, Token, TraitItem, TraitItemFn, Type,
};

pub(crate) mod args;
mod util;

use args::{InnerArgs, MyMacroArgs, MyTraitMacroArgs};
use syn::Error;
use util::{has_attr, BoolExt};

pub fn generate(args: &MyTraitMacroArgs, item: &Item) -> TokenStream {
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
        method.attrs = method
            .attrs
            .into_iter()
            .filter(|attr| !attr.path().is_ident("internal"))
            .collect::<Vec<Attribute>>();
        let method_stream = if method.default.is_none() {
            generate_trait_method(&method, name, &args)
        } else {
            method.to_token_stream()
        };
        return Some((None, method_stream));
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

fn inner_generate(
    MyTraitMacroArgs {
        default,
        ext_required,
        is_ext,
    }: &MyTraitMacroArgs,
    item: &Item,
) -> Result<TokenStream, Error> {
    let Item::Trait(input_trait) = &item else {
        return Err(Error::new(item.span(), "Input must be a trait"));
    };
    let (generated_methods, trait_methods): (Vec<_>, Vec<_>) = input_trait
        .items
        .iter()
        .zip(std::iter::repeat(input_trait))
        .filter_map(generate_method)
        .unzip();

    let trait_ident = &input_trait.ident;
    let macro_rules_name = trait_ident;
    let attrs = input_trait.attrs.as_slice();

    let mut trait_ = input_trait.clone();
    let items = trait_methods
        .into_iter()
        .map(syn::parse2)
        .collect::<Result<Vec<TraitItemFn>, _>>()?;

    let never_ident = format_ident!("{}Never", trait_ident);
    let never_ext_impl = ext_required.then_default(|| {
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
    });

    trait_.items = items.into_iter().map(TraitItem::Fn).collect();
    trait_.items.insert(
        0,
        syn::parse_quote! {
            type Impl: #trait_ident;
        },
    );

    let default_impl = default
        .as_ref()
        .map_or_else(|| quote! {$contract_name}, Ident::to_token_stream);

    let default_used = if *ext_required {
        quote! { #never_ident<$crate::#default_impl> }
    } else {
        quote! { $crate::#default_impl }
    };

    let ensure_default = default.is_none().then_default(|| {
        let message = format!(
            "The contract trait `{trait_ident}` does not provide default implementation. \
One should be passed, e.g. `#[derive_contract(Administratable(default = MyAdmin))"
        );
        quote! {
            compile_error!(#message);
        }
    });

    let extension_type = is_ext.then_default(|| {
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

    #(#attrs)*
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
            // #ensure_extension
            impl #trait_ident for $contract_name {
                type Impl = $impl_name;
            }
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

pub fn derive_contract_inner(args: &MyMacroArgs, trait_impls: &Item) -> Result<TokenStream, Error> {
    let Item::Struct(strukt) = trait_impls else {
        return Err(Error::new(trait_impls.span(), "Input must be a struct"));
    };
    let strukt_name = &strukt.ident;
    let macro_calls = args
        .args
        .iter()
        .map(|(trait_ident, InnerArgs { exts, default })| {
            if exts.is_empty() && default.is_none() {
                return quote! { #trait_ident!(#strukt_name); };
            }
            let init = default.as_ref().map_or_else(
                || quote! {#trait_ident!()},
                |default| {
                    quote! {#default }
                },
            );
            let default_impl = exts.iter().fold(
                init,
                |acc, extension| quote! { #extension<#strukt_name, #acc> },
            );
            quote! {
                #trait_ident!(#strukt_name, #default_impl);
            }
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
    fn first() {
        let input: Item = syn::parse_quote! {
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
        let result: TokenStream = generate(
            &MyTraitMacroArgs {
                default,
                ..Default::default()
            },
            &input,
        );
        println!("{}", format_snippet(&result.to_string()));

        let output = quote! {
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
        equal_tokens(&output, &result);
    }

    #[test]
    fn derive() {
        let input: Item = syn::parse_quote! {
            pub struct Contract;
        };
        let args = vec![
            (
                format_ident!("Administratable"),
                InnerArgs {
                    exts: vec![],
                    default: None,
                },
            ),
            (
                format_ident!("Upgradable"),
                InnerArgs {
                    exts: vec![format_ident!("AdministratableExt").into()],
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
        println!("{}", format_snippet(&result.to_string()));
        let output = quote! {
        pub struct Contract;
        Upgradable ! (Contract , AdministratableExt < Contract , Upgradable ! () >);
        Administratable!(Contract, Administratable!());
        };
        equal_tokens(&output, &result);
    }
}
