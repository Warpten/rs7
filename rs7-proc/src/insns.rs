use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Deref,
};

use syn::{
    DeriveInput, Expr, Ident, Lit, Meta, MetaNameValue, Token, Variant,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

struct Metadata {
    pub added: u8,
    pub removed: Option<u8>,
}

#[derive(Debug)]
struct VersionRange {
    start: u8,
    end: u8,
    instructions: BTreeSet<usize>,
}

struct NameValueList(Punctuated<MetaNameValue, Token![,]>);

impl Deref for NameValueList {
    type Target = Punctuated<MetaNameValue, Token![,]>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Parse for NameValueList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(Punctuated::parse_terminated(input)?))
    }
}

impl VersionRange {
    pub fn instructions<'a>(&'a self, instructions: &'a [(&Variant, Metadata)]) -> impl Iterator<Item = &'a Variant> {
        self.instructions.iter().map(|i| instructions[*i].0)
    }

    pub fn len(&self) -> usize {
        self.instructions.len()
    }
}

fn parse_attribute<F, R>(attrs: Option<&NameValueList>, key: &'static str, parser: F) -> Option<R>
where
    F: FnOnce(&MetaNameValue) -> Option<R>,
{
    match attrs {
        Some(a) => {
            a.0.iter()
                .find(|attr| attr.path.get_ident().unwrap().to_string() == key)
                .map(parser)
                .flatten()
        }
        None => None,
    }
}

/// Collect a map that associates version => {set of matching enum variant elements}
/// We use an indirection level by storing their index in the `instructions` vector,
/// which is constant from this point. Note that we use a `TreeSet` because in this case
/// insertion order matches traversal order, so the fact that the set sorts on insertion
/// is irrelevant.
fn collect_instruction_ranges(instructions: &[(&Variant, Metadata)]) -> Vec<VersionRange> {
    // This is the base set of instructions that is always available for the first version.
    let mut versions: BTreeMap<u8, BTreeSet<usize>> = BTreeMap::new();

    // Find the minimum and maximum versions.
    let min = instructions
        .iter()
        .map(|i| i.1.added.min(i.1.removed.unwrap_or(i.1.added)))
        .min()
        .unwrap_or(1);
    let max = instructions
        .iter()
        .map(|i| i.1.added.max(i.1.removed.unwrap_or(i.1.added)))
        .max()
        .unwrap_or(1)
        + 1;

    for version in min..=max {
        versions.entry(version).insert_entry(
            instructions
                .iter()
                .enumerate()
                .filter_map(|(i, (_, m))| {
                    if version < m.added || version >= m.removed.unwrap_or(0xFF) {
                        None
                    } else {
                        Some(i)
                    }
                })
                .collect(),
        );
    }

    // Insert an empty range for the last version

    make_stable_ranges(versions)
}

fn make_stable_ranges(versions: BTreeMap<u8, BTreeSet<usize>>) -> Vec<VersionRange> {
    // Now calculate stable ranges.
    let mut result = vec![];
    let mut current: Option<VersionRange> = None;
    for (v, entries) in versions {
        let extend = if let Some(r) = &mut current {
            r.instructions == entries
        } else {
            false
        };

        if extend {
            current.as_mut().expect("can't be empty").end += 1;
        } else {
            if let Some(r) = current {
                result.push(r);
            }

            // Start as a semi-open range
            current = Some(VersionRange {
                start: v,
                end: v + 1,
                instructions: entries,
            });
        }
    }

    if let Some(mut r) = current {
        r.end = r.start; // Transform to semi-open range
        result.push(r);
    }

    result
}

fn generate_arm<F>(v: &Variant, transform: F) -> proc_macro2::TokenStream
where
    F: FnOnce(&Ident, Vec<&Ident>) -> proc_macro2::TokenStream,
{
    let fields: Vec<_> = match &v.fields {
        syn::Fields::Named(named) => (&named.named).iter().map(|f| f.ident.as_ref().unwrap()).collect(),
        _ => panic!("Bytecode instruction only supports named fields"),
    };

    let has_bc = fields.iter().any(|f| f.to_string() == "b" || f.to_string() == "c");
    let has_d = fields.iter().any(|f| f.to_string() == "d");

    assert!(
        !(has_d && has_bc),
        "{}",
        format!(
            "Bytecode instruction {} cannot be encoded with D and B/C!",
            &v.ident.to_string().as_str()
        )
    );

    transform(&v.ident, fields)
}

pub fn bytecode_insn_impl(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    use quote::quote;
    use syn::parse2;

    let ast: DeriveInput = parse2(input).expect("Failed to parse input");

    let name = &ast.ident;

    // Extract enum variants
    let variants = match ast.data {
        syn::Data::Enum(ref data_enum) => &data_enum.variants,
        _ => panic!("Bytecode can only be used on enums"),
    };

    // Some instructions are only available on different bytecode versions.
    // And if course these instructions got injected in between others,
    // making parsing non-trivial.

    // Collect each branch and their corresponding metadata.
    let instructions = variants
        .iter()
        .map(|v| {
            let attrs = (&v.attrs)
                .iter()
                .filter(|a| a.path().is_ident("bytecode"))
                .find_map(|a| match &a.meta {
                    Meta::List(l) => l.parse_args::<NameValueList>().ok(),
                    _ => None,
                });

            let parser = |nv: &MetaNameValue| match &nv.value {
                Expr::Lit(lit) => match &lit.lit {
                    Lit::Int(i) => i.base10_parse::<u8>().ok(),
                    _ => None,
                },
                _ => None,
            };

            // LuaJIT bytecode versions start at 1 as far as I know.
            let added = parse_attribute(attrs.as_ref(), "added", parser).unwrap_or(1u8);

            // Same as `added`, but 2 is the first version where any opcode could have
            // been removed.
            let removed = parse_attribute(attrs.as_ref(), "removed", parser);

            (v, Metadata { added, removed })
        })
        .collect::<Vec<_>>();

    let versions = collect_instruction_ranges(&instructions);

    // Generate a collection of local functions
    let parsers = instructions.iter().map(|(v, _)| {
        generate_arm(v, |ident, fields| {
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

            let function_name = format!("parse_{}", ident.to_string().to_lowercase());
            let function_name = syn::Ident::new(&function_name, ident.span());

            quote! {
                #[inline] fn #function_name(insn: u32) -> #name { #name::#ident { #(#decoded_fields),* } }
            }
        })
    });

    // For each range of versions, generate an array of function pointers
    // where each element points to a lambda that parses the instruction.
    let implementations = versions
        .iter()
        .map(|version| {
            let arms = (0u32..).zip(version.instructions(&instructions)).map(|(i, v)| {
                generate_arm(v, |ident, _| {
                    let function_name = format!("parse_{}", ident.to_string().to_lowercase());
                    let function_name = syn::Ident::new(&function_name, ident.span());

                    quote! {
                        #i => #function_name(insn),
                    }
                })
            });

            let start = version.start;
            let end = version.end;

            let range_check = if start == end {
                quote! { version >= #start }
            } else {
                quote! { version >= #start && version < #end }
            };

            if version.len() == 0 {
                quote! {
                    if #range_check {
                        panic!("Unsupported bytecode version {version}.")
                    }
                }
            } else {
                quote! {
                    if #range_check {
                        return match insn & 0xFF {
                            #( #arms )*
                            _ => panic!("Unknown bytecode instruction"),
                        };
                    }
                }
            }
        })
        .rev();

    quote! {
        impl #name {
            /// Creates a new bytecode instruction.
            ///
            /// # Arguments
            ///
            /// * `data` - The instruction data to parse.
            /// * `version` - The bytecode version.
            pub fn new(data: &mut impl Buf, version: u8) -> Self {
                let insn = data.get_u32_ne();

                #( #parsers )*

                #( #implementations )*
                panic!("Bytecode version {version} is not supported");
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
        // 1. [A, C, D, AD]
        // 2. [A, B, C, AD]
        // 3. [A, B, C, AD]
        // 4. [A, B, AD]
        let output = bytecode_insn_impl(quote! {
            pub enum Instruction {
                A { a: u8 },
                #[bytecode(added = 2)]
                B { b: u8 },
                #[bytecode(removed = 4)]
                C { c: u8 },
                #[bytecode(removed = 2)]
                D { d: u16 },

                AD { a: u8, d: u16 },
            }
        });

        let expected = quote! {
            impl Instruction {
                #[doc = r" Creates a new bytecode instruction."]
                #[doc = r""]
                #[doc = r" # Arguments"]
                #[doc = r""]
                #[doc = r" * `data` - The instruction data to parse."]
                #[doc = r" * `version` - The bytecode version."]
                pub fn new(data: &mut impl Buf, version: u8) -> Self {
                    let insn = data.get_u32_ne();

                    #[inline] fn parse_a(insn: u32) -> Instruction {
                        Instruction::A { a: ((insn >> 8) & 0xFF) as u8, }
                    }
                    #[inline] fn parse_b(insn: u32) -> Instruction {
                        Instruction::B { b: ((insn >> 16) & 0xFF) as u8, }
                    }
                    #[inline] fn parse_c(insn: u32) -> Instruction {
                        Instruction::C { c: ((insn >> 24) & 0xFF) as u8, }
                    }
                    #[inline] fn parse_d(insn: u32) -> Instruction {
                        Instruction::D { d: ((insn >> 16) & 0xFFFF) as u16, }
                    }
                    #[inline] fn parse_ad(insn: u32) -> Instruction {
                        Instruction::AD {
                            a: ((insn >> 8) & 0xFF) as u8,
                            d: ((insn >> 16) & 0xFFFF) as u16,
                        }
                    }

                    if version >= 4u8 {
                        return match insn & 0xFF {
                            0u32 => parse_a(insn),
                            1u32 => parse_b(insn),
                            2u32 => parse_ad(insn),
                            _ => panic!("Unknown bytecode instruction"),
                        };
                    }
                    if version >= 2u8 && version < 4u8 {
                        return match insn & 0xFF {
                            0u32 => parse_a(insn),
                            1u32 => parse_b(insn),
                            2u32 => parse_c(insn),
                            3u32 => parse_ad(insn),
                            _ => panic!("Unknown bytecode instruction"),
                        };
                    }
                    if version >= 1u8 && version < 2u8 {
                        return match insn & 0xFF {
                            0u32 => parse_a(insn),
                            1u32 => parse_c(insn),
                            2u32 => parse_d(insn),
                            3u32 => parse_ad(insn),
                            _ => panic!("Unknown bytecode instruction"),
                        };
                    }
                    panic!("Bytecode version {version} is not supported");
                }
            }
        };
        assert_tokens_eq!(output, expected)
    }
}
