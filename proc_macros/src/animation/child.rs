use darling::FromVariant;
use syn::Ident;

#[derive(FromVariant, Debug)]
#[darling(attributes(request))]
pub struct Child {
    pub ident: Ident,
}
