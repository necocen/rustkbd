use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::{quote, TokenStreamExt};
use syn::{parse_macro_input, Data, DeriveInput, LitStr};

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
        impl rustkbd::keyboard::Layer for #name {
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

macro_rules! key {
    ($n:tt, $i:ident) => {
        ($n, quote!(rustkbd::keyboard::Key::$i))
    };
    ($i:ident) => {
        (stringify!($i), quote!(rustkbd::keyboard::Key::$i))
    };
}

#[proc_macro]
pub fn layout(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as LitStr).value();

    let table = [
        key!("", None),
        key!("Trn", Transparent),
        key!(A),
        key!(B),
        key!(C),
        key!(D),
        key!(E),
        key!(F),
        key!(G),
        key!(H),
        key!(I),
        key!(J),
        key!(K),
        key!(L),
        key!(M),
        key!(N),
        key!(O),
        key!(P),
        key!(Q),
        key!(R),
        key!(S),
        key!(T),
        key!(U),
        key!(V),
        key!(W),
        key!(X),
        key!(Y),
        key!(Z),
        key!("1", Digit1_Exclamation),
        key!("2", Digit2_At),
        key!("3", Digit3_Number),
        key!("4", Digit4_Dollar),
        key!("5", Digit5_Percent),
        key!("6", Digit6_Circumflex),
        key!("7", Digit7_Ampersand),
        key!("8", Digit8_Asterisk),
        key!("9", Digit9_LeftParenthesis),
        key!("0", Digit0_RightParenthesis),
        key!(Enter),
        key!("Esc", Escape),
        key!("Del", Delete),
        key!(Tab),
        key!(Space),
        key!("-", HyphenMinus_LowLine),
        key!("=", Equal_Plus),
        key!("[", LeftSquareBracket_LeftCurlyBracket),
        key!("]", RightSquareBracket_RightCurlyBracket),
        key!("\\", Backslash_VerticalBar),
        // key!("", NonUs_Number_Tilde),
        key!(";", Semicolon_Colon),
        key!("'", Apostrophe_Quotation),
        key!("`", Grave_Tilde),
        key!(",", Comma_LessThan),
        key!(".", Period_GreaterThan),
        key!("/", Slash_Question),
        key!("Caps", CapsLock),
        key!(F1),
        key!(F2),
        key!(F3),
        key!(F4),
        key!(F5),
        key!(F6),
        key!(F7),
        key!(F8),
        key!(F9),
        key!(F10),
        key!(F11),
        key!(F12),
        key!("PrScr", PrintScreen),
        key!("ScLck", ScrollLock),
        key!(Pause),
        key!("Ins", Insert),
        key!(Home),
        key!("PgUp", PageUp),
        key!("DelFw", DeleteForward),
        key!(End),
        key!("PgDn", PageDown),
        key!("Right", RightArrow),
        key!("Left", LeftArrow),
        key!("Down", DownArrow),
        key!("Up", UpArrow),
        key!("LCtl", LeftControl),
        key!("LSft", LeftShift),
        key!("LAlt", LeftAlt),
        key!("LGui", LeftGui),
        key!("RCtl", RightControl),
        key!("RSft", RightShift),
        key!("RAlt", RightAlt),
        key!("RGui", RightGui),
        key!("MPlay", MediaPlay),
        key!("MPau", MediaPause),
        key!("MNext", MediaNextTrack),
        key!("MPrev", MediaPrevTrack),
        key!("MStop", MediaStop),
        key!("MPlPs", MediaPlayPause),
        key!("MMute", MediaMute),
        key!("MVlUp", MediaVolumeIncrement),
        key!("MVlDn", MediaVolumeDecrement),
        key!("~", Tilde),
        key!("!", Exclamation),
        key!("@", At),
        key!("#", Hash),
        key!("$", Dollar),
        key!("%", Percent),
        key!("&", Circumflex),
        key!("^", Ampersand),
        key!("*", Asterisk),
        key!("(", LeftParenthesis),
        key!(")", RightParenthesis),
        key!("_", LowLine),
        key!("+", Plus),
        key!("{", LeftCurlyBracket),
        key!("}", RightCurlyBracket),
        key!("Pipe", VerticalBar),
        key!(":", Colon),
        key!("\"", Quotation),
        key!("<", LessThan),
        key!(">", GreaterThan),
        key!("?", Question),
    ]
    .clone()
    .into_iter()
    .collect::<HashMap<_, _>>();

    let array = input
        .trim()
        .lines()
        .map(&str::trim)
        .map(|line| {
            let array = line
                .split('|')
                .map(&str::trim)
                .collect::<Vec<_>>()
                .into_iter()
                .skip(1)
                .rev()
                .skip(1)
                .rev()
                .map(|k| {
                    if let Some(st) = table.get(k) {
                        st.clone()
                    } else {
                        let message = "layout: Unknown symbol: ".to_string() + k;
                        quote!(compile_error!(#message))
                    }
                })
                .map(|t| quote! {#t,})
                .collect::<TokenStream>();
            quote! {
                [#array]
            }
        })
        .map(|t| quote! {#t,})
        .collect::<TokenStream>();

    let expanded = quote! {
        [#array]
    };

    proc_macro::TokenStream::from(expanded)
}
