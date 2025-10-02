use darling::{ast::NestedMeta, FromMeta};

pub fn parse<T: FromMeta>(args: proc_macro::TokenStream) -> Result<T, syn::Error> {
    Ok(T::from_list(&NestedMeta::parse_meta_list(args.into())?)?)
}

#[derive(Debug, Default, FromMeta)]
pub struct TraitArgs {
    pub default_required: Option<bool>,
    pub default: Option<syn::Path>,
    pub no_impl: Option<bool>,
}

#[derive(Debug, Default, FromMeta)]
pub struct ImplArgs {
    pub default: Option<syn::Path>,
}
