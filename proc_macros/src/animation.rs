mod child;

use child::Child;
use darling::{ast, FromDeriveInput};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[derive(FromDeriveInput, Debug)]
#[darling(supports(enum_any))]
pub struct Animation {
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
    let config = animation
        .data
        .as_ref()
        .take_enum()
        .expect("Should never be a struct")
        .into_iter()
        .map(|child| {
            let ident = child.ident.to_owned();
            quote!(
                Self::#ident(animation) => animation.config(),
            )
        });
    let ui = animation
        .data
        .as_ref()
        .take_enum()
        .expect("Should never be a struct")
        .into_iter()
        .map(|child| {
            let ident = child.ident.to_owned();
            quote!(
                Self::#ident(animation) => animation.ui(ui, rendered),
            )
        });
    let shader_code = animation
        .data
        .as_ref()
        .take_enum()
        .expect("Should never be a struct")
        .into_iter()
        .map(|child| {
            let ident = child.ident.to_owned();
            quote!(
                Self::#ident(animation) => animation.shader_code(),
            )
        });
    let uses_multiple_colors = animation
        .data
        .as_ref()
        .take_enum()
        .expect("Should never be a struct")
        .into_iter()
        .map(|child| {
            let ident = child.ident.to_owned();
            quote!(
                Self::#ident(animation) => animation.uses_multiple_colors(),
            )
        });

    quote!(
        impl AnimationConfig for Animation {
            fn ui(&mut self, ui: &mut egui::Ui, rendered: egui::TextureId) -> bool {
                match self {
                    #(#ui)*
                }
            }

            fn shader_code(&self) -> std::borrow::Cow<str> {
                match self {
                    #(#shader_code)*
                }
            }

            fn config(&self) -> Config {
                match self {
                    #(#config)*
                }
            }

            fn uses_multiple_colors(&self) -> bool {
                match self {
                    #(#uses_multiple_colors)*
                }
            }
        }
    )
    .into()
}
