use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Expr, ExprLit, Fields, Lit, Type,
};

#[proc_macro_derive(Inspectable, attributes(export))]
pub fn derive_inspectable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let mut fields = vec![];

    if let Data::Struct(data_struct) = input.data {
        if let Fields::Named(fields_named) = data_struct.fields {
            for field in fields_named.named {
                let ident = field.ident.clone().unwrap();
                let field_name = ident.to_string();
                let ty = &field.ty;

                let mut kind = None;

                for attr in &field.attrs {
                    if attr.path().is_ident("export") {
                        if let Some(k) = parse_export_kind(attr) {
                            kind = Some(k);
                            break;
                        }
                    }
                }

                // fallback: if #[export] exists but no specific args, auto detect
                if kind.is_none() && field.attrs.iter().any(|a| a.path().is_ident("export")) {
                    kind = guess_kind_from_type(ty);
                }

                if let Some(kind) = kind {
                    fields.push(quote! {
                        ExportedField {
                            name: #field_name,
                            kind: #kind,
                            value: &mut self.#ident as *mut dyn std::any::Any,
                            type_id: std::any::TypeId::of::<#ty>(),
                        }
                    });
                }
            }
        }
    }

    let expanded = quote! {
        impl Inspectable for #name {
            fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
                vec![#(#fields),*]
            }
        }
    };

    TokenStream::from(expanded)
}

fn parse_export_kind(attr: &Attribute) -> Option<proc_macro2::TokenStream> {
    let mut min = None;
    let mut max = None;
    let mut has_slider = false;

    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("slider") {
            has_slider = true;
            meta.parse_nested_meta(|arg| {
                if arg.path.is_ident("min") {
                    let expr: Expr = arg.value()?.parse()?;
                    if let Expr::Lit(ExprLit { lit: Lit::Float(f), .. }) = expr {
                        min = Some(f.base10_parse::<f32>().ok());
                    }
                } else if arg.path.is_ident("max") {
                    let expr: Expr = arg.value()?.parse()?;
                    if let Expr::Lit(ExprLit { lit: Lit::Float(f), .. }) = expr {
                        max = Some(f.base10_parse::<f32>().ok());
                    }
                }
                Ok(())
            })?;
        }
        Ok(())
    });

    if has_slider {
        let min = min.flatten().unwrap_or(0.0);
        let max = max.flatten().unwrap_or(100.0);
        Some(quote! {
            ExportKind::Slider { min: #min, max: #max }
        })
    } else {
        None
    }
}

fn guess_kind_from_type(ty: &Type) -> Option<proc_macro2::TokenStream> {
    let str_ty = quote!(#ty).to_string();
    match str_ty.as_str() {
        "f32" | "f64" | "i32" | "i64" | "u32" | "u64" | "usize" | "isize" => Some(quote! {
            ExportKind::Slider { min: 0.0, max: 100.0 }
        }),
        "bool" => Some(quote! {
            ExportKind::Checkbox
        }),
        "String" => Some(quote! {
            ExportKind::Text
        }),
        _ => None,
    }
}
