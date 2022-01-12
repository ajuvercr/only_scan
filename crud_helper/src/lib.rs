#![feature(proc_macro_internals)]
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::Error, punctuated::Punctuated, spanned::Spanned, token::Comma, DeriveInput, Field,
};

const ATTRIBUTES: [&'static str; 2] = ["id", "inner"];

type Result<T> = std::result::Result<T, Error>;

#[proc_macro_derive(Builder, attributes(inner, id))]
pub fn builder_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();

    match apply_crud(ast) {
        Ok(x) => {
            println!("{}", x);
            x.into()
        }
        Err(e) => e.into_compile_error().into(),
    }
}

fn get_fields(ast: &DeriveInput) -> Result<Vec<Field>> {
    use syn::*;
    match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named, .. }),
            ..
        }) => Ok(named.iter().cloned().collect()),
        _ => Err(Error::new(
            ast.span(),
            "Only structs with named fields allowed",
        )),
    }
}

fn set_fields(ast: &mut DeriveInput, fields: Vec<Field>) -> Result<()> {
    use syn::*;
    let fields: Punctuated<Field, Comma> = fields.into_iter().collect();
    match &mut ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { ref mut named, .. }),
            ..
        }) => *named = fields,
        _ => {
            return Err(Error::new(
                ast.span(),
                "Only structs with named fields allowed",
            ))
        }
    }

    Ok(())
}

fn non_field(f: Field) -> TokenStream2 {
    let ident = &f.ident;
    quote! {
       #ident: None,
    }
}

fn apply_f<F>(fields: &Vec<Field>, f: F) -> TokenStream2
where
    F: Fn(Field) -> TokenStream2,
{
    fields.iter().cloned().map(f).collect()
}

fn new_builder(fields: &Vec<Field>) -> TokenStream2 {
    let non_fields = apply_f(fields, non_field);
    quote! {
    pub fn new() -> Self {
      Self {
        #non_fields
      }
    }}
}

fn update_function(
    fields: &Vec<Field>,
    indent: &syn::Ident,
    generics: &syn::Generics,
) -> TokenStream2 {
    let update_f = apply_f(&fields, |f| {
        let f_ident = &f.ident.unwrap();
        quote! {
          if let Some(x) = self.#f_ident {
            this.#f_ident = x;
          }
        }
    });
    quote! {
        pub fn update(self, this: &mut #indent #generics) {
            #update_f
        }
    }
}

fn update_function_rev(
    fields: &Vec<Field>,
    indent: &syn::Ident,
    generics: &syn::Generics,
) -> TokenStream2 {
    let update_f = apply_f(&fields, |f| {
        let f_ident = &f.ident.unwrap();
        quote! {
          if let Some(x) = this.#f_ident {
            self.#f_ident = x;
          }
        }
    });
    quote! {
        pub fn update(&mut self, this: #indent #generics) {
            #update_f
        }
    }
}

fn check_field(f: Field) -> TokenStream2 {
    let f_ident = &f.ident.unwrap();
    let name = f_ident.to_string();
    quote! {
      if self.#f_ident.is_none() {
        errors.push(String::from(#name));
      }
    }
}

fn set_field(f: Field) -> TokenStream2 {
    let f_ident = &f.ident.unwrap();
    quote! {
      #f_ident: self.#f_ident.unwrap(),
    }
}

fn builder_f(inp: &DeriveInput, new_ident: &syn::Ident) -> Result<TokenStream2> {
    let fields = get_fields(inp)?;
    let constructor = new_builder(&fields);

    let update_f = update_function(&fields, &inp.ident, &inp.generics);
    let update_f_rev = update_function_rev(&fields, new_ident, &inp.generics);

    let check_fields = apply_f(&fields, check_field);
    let output = apply_f(&fields, set_field);

    let indent = &inp.ident;
    let generics = &inp.generics;

    Ok(quote! {
      impl #generics #new_ident #generics {
          #constructor
          #update_f
      }

      impl #generics TryInto<#indent #generics> for #new_ident #generics {
        type Error = Vec<String>;
        fn try_into(self) -> std::result::Result<#indent #generics, Vec<String>> {
          let mut errors = Vec::new();
          #check_fields

          if(errors.is_empty()) {
              Ok(#indent {
                #output
              })
          } else {
              Err(errors)
          }
        }
      }

      impl #generics #indent #generics {
          pub fn builder() -> #new_ident #generics {
            #new_ident::new()
          }

          #update_f_rev
      }
    })
}

fn wrap_option(mut f: Field) -> Result<Field> {
    let ty = &f.ty;
    let quoted = quote! { ::std::option::Option<#ty> };
    f.ty = syn::parse(quoted.into())?;
    Ok(f)
}

fn is_not_our_attribute(attr: &syn::Attribute) -> bool {
    let f = |s| attr.path.is_ident(s);
    !ATTRIBUTES.iter().any(f)
}
fn clean_attrs(attrs: &mut Vec<syn::Attribute>) {
    attrs.retain(is_not_our_attribute);
}

fn with_field(f: Field) -> TokenStream2 {
    let ty = &f.ty;
    let f_ident = &f.ident;

    let fun_ident = syn::Ident::new(
        &format!("with_{}", f_ident.as_ref().unwrap()),
        f_ident.span(),
    );

    quote! {
       fn #fun_ident(mut self, t: #ty)-> Self {
         self.#f_ident = t.into();
         self
       }
    }
}

fn map_attributes(attributes: &mut Vec<syn::Attribute>) {
    attributes.iter_mut().for_each(|f| {
        if f.path.is_ident("inner") {
            f.path = syn::parse_str("derive").unwrap();
        };
    });
}

fn apply_crud(ast: DeriveInput) -> Result<TokenStream> {
    let fields = get_fields(&ast)?;
    let option_fields: Vec<_> = fields
        .iter()
        .cloned()
        .flat_map(wrap_option)
        .map(|mut x| {
            clean_attrs(&mut x.attrs);
            x
        })
        .collect();

    let mut new_ast = ast.clone();
    set_fields(&mut new_ast, option_fields)?;
    map_attributes(&mut new_ast.attrs);
    clean_attrs(&mut new_ast.attrs);
    new_ast.ident = syn::Ident::new(&format!("{}Builder", ast.ident), ast.ident.span());

    let builder = builder_f(&ast, &new_ast.ident)?;

    let ident = &new_ast.ident;
    let generics = &ast.generics;
    let field_impls = apply_f(&fields, with_field);

    let quoted = quote! {
        #new_ast
        #builder

     impl #generics #ident #generics {
         #field_impls
     }
    };

    Ok(quoted.into())
}
