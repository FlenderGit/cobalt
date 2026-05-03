use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, LitInt, parse_macro_input};

#[proc_macro_derive(Packet, attributes(packet, packet_field))]
pub fn derive_packet(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_packet(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_packet(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    match &input.data {
        Data::Struct(ds) => expand_packet_struct(input, ds),
        Data::Enum(data_enum) => expand_packet_enum(input, data_enum),
        _ => Err(syn::Error::new_spanned(
            input,
            "Packet can only be derived for structs or enums with named fields",
        )),
    }
}

fn expand_packet_struct(
    input: &DeriveInput,
    ds: &syn::DataStruct,
) -> syn::Result<proc_macro2::TokenStream> {
    let struct_name = &input.ident;
    let fields = match &ds.fields {
        Fields::Named(f) => &f.named,
        _ => {
            return Err(syn::Error::new_spanned(
                struct_name,
                "Only named fields supported",
            ));
        }
    };

    let packet_attr = input
        .attrs
        .iter()
        .find(|a| a.path().is_ident("packet"))
        .ok_or_else(|| syn::Error::new_spanned(input, "Missing #[packet(0x..)] attribute"))?;

    let packet_id: LitInt = packet_attr.parse_args()?;

    let serialize_calls = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        quote! {
            cobalt_protocol::Encode::encode(&self.#field_name, writer)?;
        }
    });

    let decode_fields = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        quote! {
            #field_name: cobalt_protocol::Decode::decode(reader)?
        }
    });

    Ok(quote! {
        impl cobalt_protocol::Encode for #struct_name {
            fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                #(#serialize_calls)*
                Ok(())
            }
        }

        impl cobalt_protocol::PacketId for #struct_name {
            const ID: i32 = #packet_id;
        }

        impl cobalt_protocol::Decode for #struct_name {
            fn decode<R: std::io::Read + Unpin>(reader: &mut R) -> std::io::Result<Self> {
                Ok(Self {
                    #(#decode_fields),*
                })
            }
        }

    })
}

fn expand_packet_enum(
    input: &DeriveInput,
    data_enum: &syn::DataEnum,
) -> syn::Result<proc_macro2::TokenStream> {
    let enum_name = &input.ident;
    let mut arms = Vec::new();
    let mut arms_decode = Vec::new();

    if data_enum.variants.is_empty() {
        return Err(syn::Error::new_spanned(
            input,
            "Enum must have at least one variant",
        ));
    }

    for variant in &data_enum.variants {
        // 1. Extraire #[packet(0x..)] sur la variante
        let packet_attr = variant
            .attrs
            .iter()
            .find(|a| a.path().is_ident("packet"))
            .ok_or_else(|| {
                syn::Error::new_spanned(variant, "Missing #[packet(0x..)] on variant")
            })?;

        let packet_id: LitInt = packet_attr.parse_args()?;
        let var_name = &variant.ident;

        // 2. Vérifier que ce sont des champs nommés
        let fields = match &variant.fields {
            Fields::Named(f) => &f.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    variant,
                    "Only named fields are supported in variants",
                ));
            }
        };

        let field_names: Vec<_> = fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();

        let field_encodes = fields.iter().map(|f| {
            let fname = f.ident.as_ref().unwrap();
            quote! {
                cobalt_protocol::Encode::encode(#fname, writer)?;
            }
        });

        arms.push(quote! {
            #enum_name::#var_name { #(#field_names),* } => {
                #(#field_encodes)*
                Ok(())
            }
        });

        arms_decode.push(quote! {
            #packet_id => {
                let #var_name = #enum_name::#var_name {
                    #(#field_names: cobalt_protocol::Decode::decode(reader)?),*
                };
                Ok(#var_name)
            }
        });
    }

    Ok(quote! {
        impl cobalt_protocol::Encode for #enum_name {
            fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                match self {
                    #(#arms)*
                }
            }
        }

        impl cobalt_protocol::Decode for #enum_name {
            fn decode<R: std::io::Read + Unpin>(reader: &mut R) -> std::io::Result<Self> {
                let packet_id = cobalt_protocol::types::varint::VarInt::decode(reader)?.val() as u32;
                match packet_id {
                    #(#arms_decode)*
                    id => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Unknown packet id {}", id)))
                }
            }
        }

    })
}
