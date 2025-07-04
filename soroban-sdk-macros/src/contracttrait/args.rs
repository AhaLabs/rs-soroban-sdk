use deluxe::ParseMetaItem;

pub fn parse<T: deluxe::ParseMetaItem, I: syn::parse::Parse>(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> Result<(T, I), syn::Error> {
    Ok((deluxe::parse2(args.into())?, syn::parse(item)?))
}

#[derive(deluxe::ParseMetaItem, Default)]
pub struct MyTraitMacroArgs {
    #[deluxe(default)]
    pub default: Option<syn::Ident>,
    #[deluxe(default, rename = extension_required)]
    pub ext_required: bool,
    #[deluxe(default, rename = is_extension)]
    pub is_ext: bool,
}

#[derive(deluxe::ParseMetaItem)]
pub struct MyMacroArgs {
    #[deluxe(rest)]
    pub args: std::collections::HashMap<syn::Ident, InnerArgs>,
}

#[derive(ParseMetaItem)]
pub struct InnerArgs {
    #[deluxe(append, rename = ext)]
    pub exts: Vec<syn::Path>,
    #[deluxe(default)]
    pub default: Option<syn::Ident>,
}
