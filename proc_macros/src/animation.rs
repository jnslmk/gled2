mod child;

use child::Child;
use darling::{ast, FromDeriveInput};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Ident};

#[derive(FromDeriveInput, Debug)]
#[darling(supports(enum_any))]
pub struct Animation {
    pub ident: Ident,
    pub data: ast::Data<Child, ()>,
}

pub fn derive(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    let animation = match Animation::from_derive_input(&derive_input) {
        Ok(val) => val,
        Err(err) => {
            return err.write_errors().into();
        }
    };
    let renderer = animation
        .data
        .as_ref()
        .take_enum()
        .expect("Should never be a struct")
        .into_iter()
        .map(|child| {
            let ident = child.ident.to_owned();
            quote!(
                Self::#ident(animation) => animation.renderer(),
            )
        });
    let init_gpu = animation
        .data
        .as_ref()
        .take_enum()
        .expect("Should never be a struct")
        .into_iter()
        .map(|child| {
            let ident = child.ident.to_owned();
            quote!(
                Self::#ident(animation) => animation.init_gpu(),
            )
        });

    quote!(
        impl Animation {
            pub fn renderer(&mut self) -> &AnimationRenderer {
                match self {
                    #(#renderer)*
                }
            }

            pub fn init_gpu(&mut self) {
                match self {
                    #(#init_gpu)*
                }
            }
        }
    )
    .into()
}
