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

pub trait BoolExt {
    fn then_default<T: Default>(&self, f: impl FnOnce() -> T) -> T;
}

impl BoolExt for bool {
    fn then_default<T: Default>(&self, f: impl FnOnce() -> T) -> T {
        if *self {
            f()
        } else {
            T::default()
        }
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
