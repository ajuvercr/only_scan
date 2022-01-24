#![feature(proc_macro_internals)]
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::Error, punctuated::Punctuated, spanned::Spanned, token::Comma, DeriveInput, Field,
};

const ATTRIBUTES: [&'static str; 5] = [
    "inner_builder",
    "inner_new",
    "no_builder",
    "use_default",
    "no_new",
];

type Result<T> = std::result::Result<T, Error>;

fn has_attr<I: ?Sized>(attrs: &Vec<syn::Attribute>, i: &I) -> bool
where
    syn::Ident: PartialEq<I>,
{
    attrs.iter().find(|f| f.path.is_ident(i)).is_some()
}

#[proc_macro_derive(Builder, attributes(inner_builder, no_builder, use_default))]
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

#[proc_macro_derive(New, attributes(inner_new, no_new))]
pub fn new_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    match apply_new(ast) {
        Ok(x) => x.into(),
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
    let use_default = has_attr(&f.attrs, "use_default");
    if use_default {
        quote! {}
    } else {
        quote! {
          if self.#f_ident.is_none() {
            errors.push(String::from(#name));
          }
        }
    }
}

fn set_field(f: Field) -> Option<TokenStream2> {
    let f_ident = &f.ident.unwrap();
    let use_default = has_attr(&f.attrs, "use_default");
    let no_builder = has_attr(&f.attrs, "no_builder");
    match (no_builder, use_default) {
        (true, false) => None,
        (true, true) => quote! { #f_ident: Default::default(), }.into(),
        (false, false) => quote! { #f_ident: self.#f_ident.unwrap(), }.into(),
        (false, true) => quote! { #f_ident: self.#f_ident.unwrap_or_default(), }.into(),
    }
}

fn builder_f(inp: &DeriveInput, new_ident: &syn::Ident) -> Result<TokenStream2> {
    let all_fields = get_fields(inp)?;
    let fields: Vec<_> = all_fields
        .iter()
        .filter(|f| !has_attr(&f.attrs, "no_builder"))
        .cloned()
        .collect();
    let constructor = new_builder(&fields);

    let update_f = update_function(&fields, &inp.ident, &inp.generics);
    let update_f_rev = update_function_rev(&fields, new_ident, &inp.generics);

    let check_fields = apply_f(&fields, check_field);

    let indent = &inp.ident;
    let generics = &inp.generics;

    let try_into_impl = if let Some(output) = all_fields
        .iter()
        .cloned()
        .try_fold(quote! {}, |q, f| set_field(f).map(|q1| quote! { #q1 #q }))
    {
        quote! {
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
        }
    } else {
        quote! {}
    };

    //    fields.iter().cloned().map(f).collect()

    Ok(quote! {
      impl #generics #new_ident #generics {
          #constructor
          #update_f
      }

      #try_into_impl


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

    if has_attr(&f.attrs, "no_builder") {
        quote! {}
    } else {
        quote! {
           fn #fun_ident(mut self, t: #ty)-> Self {
             self.#f_ident = t.into();
             self
           }
        }
    }
}

fn get_inner_attr(attributes: &Vec<syn::Attribute>, path: &str) -> Option<TokenStream2> {
    attributes
        .iter()
        .filter(|f| f.path.is_ident(path))
        .flat_map(|x| x.parse_args())
        .next()
}

fn apply_crud(ast: DeriveInput) -> Result<TokenStream> {
    let fields = get_fields(&ast)?;
    let option_fields: Vec<_> = fields
        .iter()
        .filter(|f| !has_attr(&f.attrs, "no_builder"))
        .cloned()
        .flat_map(wrap_option)
        .map(|mut x| {
            clean_attrs(&mut x.attrs);
            x
        })
        .collect();

    let mut new_ast = ast.clone();
    set_fields(&mut new_ast, option_fields)?;

    let inner_attr = get_inner_attr(&new_ast.attrs, "inner_builder");
    new_ast.attrs = Vec::new();
    clean_attrs(&mut new_ast.attrs);
    new_ast.ident = syn::Ident::new(&format!("{}Builder", ast.ident), ast.ident.span());

    let builder = builder_f(&ast, &new_ast.ident)?;

    let ident = &new_ast.ident;
    let generics = &ast.generics;
    let field_impls = {
        let nfs: Vec<_> = fields
            .iter()
            .filter(|f| !has_attr(&f.attrs, "no_builder"))
            .cloned()
            .collect();
        apply_f(&nfs, with_field)
    };

    let quoted = quote! {
        #inner_attr
        #new_ast
        #builder

     impl #generics #ident #generics {
         #field_impls
     }
    };

    Ok(quoted.into())
}

fn apply_new(ast: DeriveInput) -> Result<TokenStream> {
    let mut new_ast = ast.clone();

    let fields = get_fields(&ast)?;
    let new_fields: Vec<_> = fields
        .iter()
        .filter(|f| !has_attr(&f.attrs, "no_new"))
        .cloned()
        .map(|mut x| {
            clean_attrs(&mut x.attrs);
            x
        })
        .collect();

    let inner_attr = get_inner_attr(&new_ast.attrs, "inner_new");
    new_ast.attrs = Vec::new();
    clean_attrs(&mut new_ast.attrs);
    new_ast.ident = syn::Ident::new(&format!("{}New", ast.ident), ast.ident.span());
    set_fields(&mut new_ast, new_fields)?;

    let quoted = quote! {
        #inner_attr
        #new_ast
    };

    Ok(quoted.into())
}
