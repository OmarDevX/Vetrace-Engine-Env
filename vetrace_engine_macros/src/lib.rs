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


/// Derives serialized variant metadata for a unit enum used by the generic
/// inspector and Lua reflection layers.
#[proc_macro_derive(VetraceEnum, attributes(serde))]
pub fn derive_vetrace_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_vetrace_enum(input) {
        Ok(tokens) => TokenStream::from(tokens),
        Err(error) => TokenStream::from(error.to_compile_error()),
    }
}

fn expand_vetrace_enum(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    use syn::LitStr;

    let rename_all = serde_rename_all(&input.attrs)?;
    let name = input.ident;
    let variants = match input.data {
        Data::Enum(data) => data.variants,
        _ => return Err(syn::Error::new_spanned(name, "VetraceEnum supports enums only")),
    };

    let mut serialized = Vec::new();
    for variant in variants {
        let syn::Variant { attrs, ident, fields, .. } = variant;
        if !matches!(fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                ident,
                "VetraceEnum supports unit variants only",
            ));
        }
        let renamed = serde_variant_rename(&attrs)?
            .unwrap_or_else(|| apply_serde_rename_all(&ident.to_string(), rename_all.as_deref()));
        serialized.push(LitStr::new(&renamed, ident.span()));
    }

    Ok(quote! {
        impl ::vetrace_core::reflection::VetraceEnum for #name {
            fn variants() -> &'static [&'static str] {
                &[#(#serialized),*]
            }
        }
    })
}

fn serde_rename_all(attributes: &[Attribute]) -> syn::Result<Option<String>> {
    let mut result = None;
    for attribute in attributes {
        if !attribute.path().is_ident("serde") { continue; }
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename_all") {
                result = Some(meta.value()?.parse::<syn::LitStr>()?.value());
            }
            Ok(())
        })?;
    }
    Ok(result)
}

fn serde_variant_rename(attributes: &[Attribute]) -> syn::Result<Option<String>> {
    let mut result = None;
    for attribute in attributes {
        if !attribute.path().is_ident("serde") { continue; }
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                result = Some(meta.value()?.parse::<syn::LitStr>()?.value());
            }
            Ok(())
        })?;
    }
    Ok(result)
}

fn apply_serde_rename_all(value: &str, rename_all: Option<&str>) -> String {
    let words = split_identifier_words(value);
    match rename_all {
        Some("lowercase") => words.concat().to_ascii_lowercase(),
        Some("UPPERCASE") => words.concat().to_ascii_uppercase(),
        Some("snake_case") => words.join("_").to_ascii_lowercase(),
        Some("SCREAMING_SNAKE_CASE") => words.join("_").to_ascii_uppercase(),
        Some("kebab-case") => words.join("-").to_ascii_lowercase(),
        Some("SCREAMING-KEBAB-CASE") => words.join("-").to_ascii_uppercase(),
        Some("camelCase") => {
            let mut iter = words.into_iter();
            let mut result = iter.next().unwrap_or_default().to_ascii_lowercase();
            for word in iter { result.push_str(&capitalize(&word)); }
            result
        }
        Some("PascalCase") => words.into_iter().map(|word| capitalize(&word)).collect::<Vec<_>>().concat(),
        _ => value.to_owned(),
    }
}

fn split_identifier_words(value: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let chars = value.chars().collect::<Vec<_>>();
    for (index, character) in chars.iter().copied().enumerate() {
        let previous = index.checked_sub(1).and_then(|i| chars.get(i)).copied();
        let next = chars.get(index + 1).copied();
        let boundary = character == '_'
            || (character.is_uppercase()
                && !current.is_empty()
                && (previous.map(char::is_lowercase).unwrap_or(false)
                    || next.map(char::is_lowercase).unwrap_or(false)));
        if boundary && !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
        if character != '_' { current.push(character); }
    }
    if !current.is_empty() { words.push(current); }
    words
}

fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else { return String::new(); };
    first.to_uppercase().collect::<String>() + &chars.as_str().to_ascii_lowercase()
}

/// Derives the schema metadata used by Vetrace's generic component registry.
///
/// The component itself remains a normal Rust ECS component and must also
/// implement `Clone`, `Default`, `serde::Serialize`, and
/// `serde::de::DeserializeOwned` (normally through derives).
///
/// ```ignore
/// #[derive(Clone, Default, Serialize, Deserialize, VetraceComponent)]
/// #[vetrace_component(id = "my_game.health", display_name = "Health", category = "Gameplay")]
/// struct Health {
///     #[vetrace(min = 0.0, max = 100.0)]
///     current: f32,
/// }
/// ```
#[proc_macro_derive(VetraceComponent, attributes(vetrace_component, vetrace))]
pub fn derive_vetrace_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_vetrace_component(input) {
        Ok(tokens) => TokenStream::from(tokens),
        Err(error) => TokenStream::from(error.to_compile_error()),
    }
}

fn expand_vetrace_component(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    use syn::{LitBool, LitStr};

    let name = input.ident;
    let generics = input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut stable_id = None::<LitStr>;
    let mut display_name = None::<LitStr>;
    let mut category = None::<LitStr>;
    let mut description = None::<LitStr>;
    let mut constructible = true;
    let mut removable = true;
    let mut lua_accessible = true;

    for attribute in &input.attrs {
        if !attribute.path().is_ident("vetrace_component") { continue; }
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("id") {
                stable_id = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("display_name") {
                display_name = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("category") {
                category = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("description") {
                description = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("constructible") {
                constructible = meta.value()?.parse::<LitBool>()?.value;
            } else if meta.path.is_ident("removable") {
                removable = meta.value()?.parse::<LitBool>()?.value;
            } else if meta.path.is_ident("lua_accessible") {
                lua_accessible = meta.value()?.parse::<LitBool>()?.value;
            } else if meta.path.is_ident("non_constructible") {
                constructible = false;
            } else if meta.path.is_ident("non_removable") {
                removable = false;
            } else if meta.path.is_ident("hidden_from_lua") {
                lua_accessible = false;
            } else {
                return Err(meta.error("unsupported vetrace_component option"));
            }
            Ok(())
        })?;
    }

    let stable_id = stable_id.ok_or_else(|| syn::Error::new_spanned(
        &name,
        "VetraceComponent requires #[vetrace_component(id = \"namespace.component\")]",
    ))?;
    let display_name = display_name.unwrap_or_else(|| LitStr::new(&humanize_macro_name(&name.to_string()), name.span()));
    let category = category.unwrap_or_else(|| LitStr::new("General", name.span()));

    let fields = match input.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,
            _ => return Err(syn::Error::new_spanned(&name, "VetraceComponent currently supports structs with named fields")),
        },
        _ => return Err(syn::Error::new_spanned(&name, "VetraceComponent currently supports structs only")),
    };

    let mut field_builders = Vec::new();
    for field in fields {
        let syn::Field { ident, ty, attrs, .. } = field;
        let ident = ident.expect("named fields have identifiers");
        let field_name = ident.to_string();
        let mut skip = false;
        let mut read_only = false;
        let mut runtime_only = false;
        let mut hidden_from_lua = false;
        let mut field_display_name = None::<LitStr>;
        let mut field_description = None::<LitStr>;
        let mut field_kind = None::<LitStr>;
        let mut enum_options = false;
        let mut min = None::<proc_macro2::TokenStream>;
        let mut max = None::<proc_macro2::TokenStream>;
        let mut step = None::<proc_macro2::TokenStream>;

        for attribute in &attrs {
            if !attribute.path().is_ident("vetrace") { continue; }
            attribute.parse_nested_meta(|meta| {
                if meta.path.is_ident("skip") {
                    skip = true;
                } else if meta.path.is_ident("read_only") {
                    read_only = true;
                } else if meta.path.is_ident("runtime_only") {
                    runtime_only = true;
                } else if meta.path.is_ident("hidden_from_lua") {
                    hidden_from_lua = true;
                } else if meta.path.is_ident("display_name") {
                    field_display_name = Some(meta.value()?.parse()?);
                } else if meta.path.is_ident("description") {
                    field_description = Some(meta.value()?.parse()?);
                } else if meta.path.is_ident("kind") {
                    field_kind = Some(meta.value()?.parse()?);
                } else if meta.path.is_ident("enum_options") {
                    enum_options = true;
                } else if meta.path.is_ident("min") {
                    let expression: Expr = meta.value()?.parse()?;
                    min = Some(quote!(Some((#expression) as f64)));
                } else if meta.path.is_ident("max") {
                    let expression: Expr = meta.value()?.parse()?;
                    max = Some(quote!(Some((#expression) as f64)));
                } else if meta.path.is_ident("step") {
                    let expression: Expr = meta.value()?.parse()?;
                    step = Some(quote!(Some((#expression) as f64)));
                } else {
                    return Err(meta.error("unsupported vetrace field option"));
                }
                Ok(())
            })?;
        }
        if skip { continue; }

        let display_chain = field_display_name.map(|value| quote!(field = field.with_display_name(#value);));
        let description_chain = field_description.map(|value| quote!(field = field.with_description(#value);));
        let kind_chain = match field_kind {
            Some(kind) => {
                let variant = field_kind_variant(&kind.value()).ok_or_else(|| {
                    syn::Error::new_spanned(kind, "unsupported field kind; expected bool, integer, unsigned, number, string, vec2, vec3, vec4, quaternion, color, enum, asset_path, entity_reference, array, object, or unknown")
                })?;
                Some(quote!(field = field.with_kind(::vetrace_core::reflection::FieldKind::#variant);))
            }
            None => None,
        };
        let enum_chain = enum_options.then(|| quote!(
            field = field.with_enum_variants(
                <#ty as ::vetrace_core::reflection::VetraceEnum>::variants().iter().copied(),
            );
        ));
        let read_only_chain = read_only.then(|| quote!(field = field.read_only();));
        let runtime_only_chain = runtime_only.then(|| quote!(field = field.runtime_only();));
        let hidden_chain = hidden_from_lua.then(|| quote!(field = field.hidden_from_lua();));
        let range_chain = if min.is_some() || max.is_some() || step.is_some() {
            let min = min.unwrap_or_else(|| quote!(None));
            let max = max.unwrap_or_else(|| quote!(None));
            let step = step.unwrap_or_else(|| quote!(None));
            Some(quote!(field = field.with_range(#min, #max, #step);))
        } else {
            None
        };

        field_builders.push(quote!({
            let mut field = ::vetrace_core::reflection::FieldSchema::inferred(
                #field_name,
                &default_component.#ident,
            );
            #display_chain
            #description_chain
            #kind_chain
            #enum_chain
            #read_only_chain
            #runtime_only_chain
            #hidden_chain
            #range_chain
            field
        }));
    }

    let description_chain = description.map(|value| quote!(schema.description = Some(#value.to_owned());));

    Ok(quote! {
        impl #impl_generics ::vetrace_core::reflection::VetraceComponent for #name #ty_generics #where_clause {
            const STABLE_ID: &'static str = #stable_id;
            const DISPLAY_NAME: &'static str = #display_name;
            const CATEGORY: &'static str = #category;

            fn component_schema() -> ::vetrace_core::reflection::ComponentSchema {
                let default_component = <Self as ::core::default::Default>::default();
                let mut schema = ::vetrace_core::reflection::ComponentSchema::new(
                    #stable_id,
                    #display_name,
                    #category,
                );
                schema.constructible = #constructible;
                schema.removable = #removable;
                schema.lua_accessible = #lua_accessible;
                #description_chain
                schema.fields = vec![#(#field_builders),*];
                schema
            }
        }
    })
}

fn field_kind_variant(value: &str) -> Option<syn::Ident> {
    let variant = match value.to_ascii_lowercase().as_str() {
        "null" => "Null",
        "bool" | "boolean" => "Boolean",
        "int" | "integer" => "Integer",
        "uint" | "unsigned" | "unsigned_integer" => "UnsignedInteger",
        "float" | "number" => "Number",
        "string" => "String",
        "vec2" => "Vec2",
        "vec3" => "Vec3",
        "vec4" => "Vec4",
        "quat" | "quaternion" => "Quaternion",
        "color" => "Color",
        "enum" => "Enum",
        "asset" | "asset_path" => "AssetPath",
        "entity" | "entity_reference" => "EntityReference",
        "array" => "Array",
        "object" => "Object",
        "unknown" => "Unknown",
        _ => return None,
    };
    Some(syn::Ident::new(variant, proc_macro2::Span::call_site()))
}

fn humanize_macro_name(value: &str) -> String {
    let mut result = String::new();
    let mut previous_lowercase = false;
    for character in value.chars() {
        if character == '_' {
            result.push(' ');
            previous_lowercase = false;
            continue;
        }
        if character.is_uppercase() && previous_lowercase { result.push(' '); }
        result.push(character);
        previous_lowercase = character.is_lowercase();
    }
    result
}
