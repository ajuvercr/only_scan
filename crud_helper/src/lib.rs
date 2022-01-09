#![feature(proc_macro_internals)]
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
    parse::Error,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Comma, Token},
    DeriveInput, Field,
};

type Result<T> = std::result::Result<T, Error>;

#[proc_macro_derive(Crud, attributes(id))]
pub fn foo(input: TokenStream) -> TokenStream {
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
    new_ast.ident = syn::Ident::new(&format!("{}Patch", ast.ident), ast.ident.span());

    let mut quoted = quote! {#new_ast};
    quoted.extend(fields.iter().map(|f| with_field(&new_ast, f)));
    Ok(quoted.into())
}

#[cfg(test)]
mod tests {}
