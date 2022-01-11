#![feature(proc_macro_internals)]
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
    parse::Error, punctuated::Punctuated, spanned::Spanned, token::Comma, DeriveInput, Field,
};

type Result<T> = std::result::Result<T, Error>;

#[proc_macro_derive(Crud, attributes(inner, id))]
pub fn foo(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();

    let attrs = ast
        .attrs
        .iter()
        .map(|x| x.to_token_stream())
        .collect::<Vec<_>>();
    println!("{:?}", attrs);

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

fn builder_f(inp: &DeriveInput, new_ident: &syn::Ident) -> Result<TokenStream2> {
    let fields = get_fields(inp)?;
    let non_fields: TokenStream2 = fields.iter().cloned().map(non_field).collect();

    let check_fields: TokenStream2 = fields
        .iter()
        .cloned()
        .map(|f| {
            let f_ident = &f.ident.unwrap();
            let name = f_ident.to_string();
            quote! {
              if self.#f_ident.is_none() {
                errors.push(String::from(#name));
              }
            }
        })
        .collect();

    let output: TokenStream2 = fields
        .iter()
        .cloned()
        .map(|f| {
            let f_ident = &f.ident;
            quote! {
              #f_ident: self.#f_ident.unwrap(),
            }
        })
        .collect();

    let update_f: TokenStream2 = fields
        .iter()
        .cloned()
        .map(|f| {
            let f_ident = &f.ident;
            quote! {
                if let Some(x) = self.#f_ident {
                  this.#f_ident = x;
                }
            }
        })
        .collect();

    let indent = &inp.ident;
    let generics = &inp.generics;

    Ok(quote! {
      impl #generics #new_ident #generics {
        pub fn new() -> Self {
          Self {
            #non_fields
          }
        }

        pub fn update(self, this: &mut #indent #generics) {
            #update_f

        }
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
      }
    })
}

fn wrap_option(mut f: Field) -> Result<Field> {
    let ty = &f.ty;
    let quoted = quote! { ::std::option::Option<#ty> };
    let nt = syn::parse(quoted.into())?;
    f.ty = nt;
    Ok(f)
}

fn clean_field(mut f: Field) -> Field {
    f.attrs = vec![];
    f
}

fn with_field(ast: &DeriveInput, f: &Field) -> TokenStream2 {
    let ident = &ast.ident;
    let generics = &ast.generics;
    let ty = &f.ty;
    let f_ident = &f.ident;

    let fun_ident = syn::Ident::new(
        &format!("with_{}", f_ident.as_ref().unwrap()),
        f_ident.span(),
    );

    quote! {
      impl #generics #ident #generics {
        fn #fun_ident(mut self, t: #ty)-> Self {
          self.#f_ident = t.into();
          self
        }
      }
    }
}

fn map_attributes(attributes: &Vec<syn::Attribute>) -> Option<syn::Attribute> {
    attributes.iter().find_map(|f| {
        if f.path.is_ident("inner") {
            let mut nf = f.clone();
            nf.path = syn::parse_str("derive").unwrap();
            Some(nf)
        } else {
            None
        }
    })
}

fn apply_crud(ast: DeriveInput) -> Result<TokenStream> {
    let fields = get_fields(&ast)?;
    let option_fields: Vec<_> = fields
        .iter()
        .cloned()
        .flat_map(wrap_option)
        .map(clean_field)
        .collect();

    let mut new_ast = ast.clone();
    set_fields(&mut new_ast, option_fields)?;
    new_ast.attrs = map_attributes(&ast.attrs).into_iter().collect();

    new_ast.ident = syn::Ident::new(&format!("{}Builder", ast.ident), ast.ident.span());

    let builder = builder_f(&ast, &new_ast.ident)?;

    let mut quoted = quote! {#new_ast #builder};
    quoted.extend(fields.iter().map(|f| with_field(&new_ast, f)));

    Ok(quoted.into())
}
