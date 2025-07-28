use proc_macro2::TokenStream;
use std::io::{Read, Write};

#[allow(unused)]
pub(crate) fn equal_tokens(expected: &TokenStream, actual: &TokenStream) {
    assert_eq!(
        format_snippet(&expected.to_string()),
        format_snippet(&actual.to_string())
    );
}

pub(crate) fn p_e(e: std::io::Error) -> std::io::Error {
    eprintln!("{e:#?}");
    e
}

pub(crate) fn has_attr(attrs: &[syn::Attribute], ident_str: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(ident_str))
}

pub fn arg_to_ident(arg: &syn::FnArg) -> Option<&syn::Ident> {
    if let syn::FnArg::Typed(syn::PatType { pat, .. }) = arg {
        if let syn::Pat::Ident(pat_ident) = &**pat {
            return Some(&pat_ident.ident);
        }
    }
    None
}

pub fn args_to_idents(
    inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::Token!(,)>,
) -> Vec<&syn::Ident> {
    inputs.iter().filter_map(arg_to_ident).collect::<Vec<_>>()
}

pub fn is_env(path: &syn::TypePath) -> bool {
    path.path
        .segments
        .last()
        .map(|seg| seg.ident == "Env")
        .unwrap_or(false)
}

pub fn is_env_reference(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Reference(syn::TypeReference { elem, .. }) => {
            matches!(elem.as_ref(), syn::Type::Path(path) if is_env(path))
        }
        _ => false,
    }
}

/// Format the given snippet. The snippet is expected to be *complete* code.
/// When we cannot parse the given snippet, this function returns `None`.
#[allow(unused)]
pub(crate) fn format_snippet(snippet: &str) -> String {
    let mut child = std::process::Command::new("rustfmt")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(snippet.as_bytes())
        .map_err(p_e)
        .unwrap();
    child.wait().unwrap();
    let mut buf = String::new();
    child.stdout.unwrap().read_to_string(&mut buf).unwrap();
    buf
}
