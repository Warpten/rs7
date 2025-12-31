use syn::DeriveInput;

pub fn bytecode_insn_impl(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    use quote::quote;
    use syn::parse2;

    let ast: DeriveInput = parse2(input).expect("Failed to parse input");

    let name = &ast.ident;

    // Extract enum variants
    let variants = match ast.data {
        syn::Data::Enum(ref data_enum) => &data_enum.variants,
        _ => panic!("EnumIter can only be used on enums"),
    };

    let arms = (0u32..).zip(variants.iter()).map(|(i, v)| {
        let ident = &v.ident;

        let fields: Vec<_> = match &v.fields {
            syn::Fields::Named(named) => (&named.named)
                .iter()
                .map(|f| f.ident.as_ref().unwrap())
                .collect(),
            _ => panic!("Bytecode instruction only supports named fields"),
        };

        let has_bc = fields
            .iter()
            .any(|f| f.to_string() == "b" || f.to_string() == "c");
        let has_d = fields.iter().any(|f| f.to_string() == "d");

        assert!(
            !(has_d && has_bc),
            "{}",
            format!(
                "Bytecode instruction {} cannot be encoded with D and B/C!",
                ident.to_string().as_str()
            )
        );

        let decoded_fields = fields.iter().map(|f| {
            let expr = match f.to_string().as_str() {
                "a" => quote! { ((insn >> 8) & 0xFF) as u8 },
                "b" => quote! { ((insn >> 16) & 0xFF) as u8 },
                "c" => quote! { ((insn >> 24) & 0xFF) as u8 },
                "d" => quote! { ((insn >> 16) & 0xFFFF) as u16 },
                other => panic!("Unknown field '{}': expected a, b, c, or d", other),
            };

            quote! { #f: #expr }
        });

        quote! { #i => Self::#ident { #(#decoded_fields),* }, }
    });

    quote! {
        impl #name {
            pub fn new(data: &mut impl Buf) -> Self {
                let insn = data.get_u32_ne();

                match insn & 0xFF {
                    #( #arms )*
                    _ => panic!("Unknown bytecode instruction"),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::insns::bytecode_insn_impl;

    use assert_tokens_eq::assert_tokens_eq;
    use quote::quote;

    #[test]
    #[should_panic]
    pub fn invalid_codegen() {
        _ = bytecode_insn_impl(quote! {
            pub enum Instruction {
                A { a: u8 },
                // This is invalid; D is (B << 8) | C.
                BD { b: u8, d: u16 },
            }
        });
    }

    #[test]
    pub fn valid_codegen() {
        let output = bytecode_insn_impl(quote! {
            pub enum Instruction {
                A { a: u8 },
                B { b: u8 },
                C { c: u8 },
                D { d: u16 },

                AD { a: u8, d: u16 },
            }
        });

        let expected = quote! {
            impl Instruction {
                pub fn new(data: &mut impl Buf) -> Self {
                    let insn = data.get_u32_ne();
                    match insn & 0xFF {
                        0u32 => Self::A { a: ((insn >> 8) & 0xFF) as u8 },
                        1u32 => Self::B { b: ((insn >> 16) & 0xFF) as u8 },
                        2u32 => Self::C { c: ((insn >> 24) & 0xFF) as u8 },
                        3u32 => Self::D { d: ((insn >> 16) & 0xFFFF) as u16 },
                        4u32 => Self::AD {
                            a: ((insn >> 8) & 0xFF) as u8,
                            d: ((insn >> 16) & 0xFFFF) as u16,
                        },
                        _ => panic!("Unknown bytecode instruction"),
                    }
                }
            }
        };
        assert_tokens_eq!(output, expected)
    }
}
