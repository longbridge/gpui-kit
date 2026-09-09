use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

/// Resolve the GPUI API exposed to the crate where a macro is expanded.
///
/// `gpui-kit` is preferred because it re-exports GPUI and is the only direct
/// dependency required by kit consumers. The `gpui-pre` package fallback
/// preserves standalone `gpui-component` consumers, including dependencies
/// that rename that package to `gpui` (the conventional name).
pub(crate) fn gpui() -> syn::Result<TokenStream> {
    match crate_name("gpui-kit") {
        Ok(found) => Ok(found_crate_path(found)),
        Err(kit_error) => crate_name("gpui-pre")
            .map(found_crate_path)
            .map_err(|gpui_error| {
                syn::Error::new(
                    Span::call_site(),
                    format!(
                        "IntoPlot requires a direct dependency on `gpui-kit` or `gpui-pre`: \
                         gpui-kit lookup failed: {kit_error}; gpui-pre lookup failed: {gpui_error}"
                    ),
                )
            }),
    }
}

fn found_crate_path(found: FoundCrate) -> TokenStream {
    match found {
        FoundCrate::Itself => quote!(crate),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(::#ident)
        }
    }
}
