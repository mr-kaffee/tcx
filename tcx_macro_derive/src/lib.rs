use convert_case::{Casing, Case};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, format_ident};
use syn::{self, spanned::Spanned, Data, Error, Fields};

#[proc_macro_derive(AsRefStr)]
pub fn as_ref_macro_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    impl_as_ref_macro(&ast)
}

fn impl_as_ref_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let data = &ast.data;

    match data {
        Data::Enum(data_enum) => {
            let mut match_body = TokenStream2::new();
            for variant in &data_enum.variants {
                match variant.fields {
                    Fields::Unit => {
                        let variant_name = &variant.ident;
                        match_body.extend(quote! {
                            #name::#variant_name => stringify!(#variant_name),
                        })
                    }
                    _ => {
                        return Error::new(
                            variant.span(),
                            "AsRefStr is only supported on enums with variants without any fields",
                        )
                        .into_compile_error()
                        .into()
                    }
                }
            }

            let as_ref_impl = quote! {
                impl AsRef<str> for #name {
                    fn as_ref(&self) -> &str {
                        match (self) {
                            #match_body
                        }
                    }
                }
            };

            as_ref_impl.into()
        }
        _ => Error::new(name.span(), "AsRefStr is only supported on enum types")
            .into_compile_error()
            .into(),
    }
}

#[proc_macro_derive(ConstArray)]
pub fn const_array_macro_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    impl_const_array_macro(&ast)
}

fn impl_const_array_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let data = &ast.data;

    match data {
        Data::Enum(data_enum) => {
            let mut size = 0usize;
            let mut array_body = TokenStream2::new();
            for variant in &data_enum.variants {
                match variant.fields {
                    Fields::Unit => {
                        let variant_name = &variant.ident;
                        array_body.extend(quote! {
                            #name::#variant_name,
                        });
                        size += 1;
                    }
                    _ => {
                        return Error::new(
                            variant.span(),
                            "ConstArray is only supported on enums with variants without any fields",
                        )
                        .into_compile_error()
                        .into()
                    }
                }
            }

            let const_name = format_ident!("{}", name.to_string().to_case(Case::UpperSnake));

            let const_array = quote! {
                pub const #const_name: [#name; #size] = [
                    #array_body
                ];
            };

            const_array.into()
        }
        _ => Error::new(name.span(), "ConstArray is only supported on enum types")
            .into_compile_error()
            .into(),
    }
}

