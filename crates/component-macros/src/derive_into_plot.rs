use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

pub fn derive_into_plot(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let type_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = ast.generics.split_for_impl();
    let gpui = match crate::crate_path::gpui() {
        Ok(path) => path,
        Err(error) => return error.into_compile_error().into(),
    };
    let component = match crate::crate_path::component() {
        Ok(path) => path,
        Err(error) => return error.into_compile_error().into(),
    };

    // The element behind every plot lives in Base; the derive only names it.
    let expanded = quote! {
        impl #impl_generics #gpui::IntoElement for #type_name #type_generics #where_clause {
            type Element = #component::plot::PlotElement<Self>;

            fn into_element(self) -> Self::Element {
                #component::plot::PlotElement::new(self)
            }
        }
    };

    TokenStream::from(expanded)
}
