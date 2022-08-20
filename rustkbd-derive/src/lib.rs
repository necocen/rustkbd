use quote::{quote, TokenStreamExt};
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(Layer)]
pub fn derive_layer(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = ast.ident;
    let variants = match ast.data {
        Data::Enum(ref data) => data.variants.iter().map(|v| &v.ident),
        _ => unimplemented!(),
    }
    .map(|v| {
        quote! {
            #name::#v,
        }
    })
    .reduce(|mut acc, v| {
        acc.append_all(v);
        acc
    });

    let expanded = quote! {
        impl rustkbd_core::keyboard::Layer for #name {
            fn below(&self) -> Option<Self> {
                let layers = [#variants];
                layers
                    .iter()
                    .enumerate()
                    .find(|(_, l)| l == &self)
                    .and_then(|(i, _)| if i > 0 { layers.get(i - 1) } else { None })
                    .copied()
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}
