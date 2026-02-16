// 该文件是 Shanan （山南西风） 项目的一部分。
// src/lib.rs - 过程宏库主文件
//
// 本文件根据 Apache 许可证第 2.0 版（以下简称“许可证”）授权使用；
// 除非遵守该许可证条款，否则您不得使用本文件。
// 您可通过以下网址获取许可证副本：
// http://www.apache.org/licenses/LICENSE-2.0
// 除非适用法律要求或书面同意，根据本许可协议分发的软件均按“原样”提供，
// 不附带任何形式的明示或暗示的保证或条件。
// 有关许可权限与限制的具体条款，请参阅本许可协议。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, Wareless Group

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::collections::HashMap;
use std::fs;
use syn::{DeriveInput, Ident, parse_macro_input};

///
/// 一个根据 TOML 文件生成标签枚举的宏。
/// 参数格式：`file = "path/to/labels.toml"`
/// 如果 toml 文件是这样的：
/// ```toml
/// cat = 0
/// dog = 1
/// ```
/// 然后代码是
/// ```rust
/// #[toml_label(file = "labels.toml")]
/// pub enum MyLabel;
/// ```
/// 则最终生成的代码如下：
/// ```rust
/// pub enum MyLabel {
///   Cat = 0,
///   Dog = 1,
/// }
/// ```
///
#[proc_macro_attribute]
pub fn toml_label(args: TokenStream, input: TokenStream) -> TokenStream {
  let args_str = args.to_string();
  let parts: Vec<&str> = args_str.split(',').collect();

  if parts.len() != 1 {
    return syn::Error::new(Span::call_site(), "Expected format: file = \"path\"")
      .to_compile_error()
      .into();
  }

  let file_arg = parts[0].trim();

  if !file_arg.trim().starts_with("file") || !file_arg.contains("=") {
    return syn::Error::new(
      proc_macro2::Span::call_site(),
      "First argument must be file path",
    )
    .to_compile_error()
    .into();
  }

  let file_path = { file_arg.split('=').nth(1).unwrap().trim().trim_matches('"') };

  let toml_content = match fs::read_to_string(file_path) {
    Ok(content) => content,
    Err(e) => {
      return syn::Error::new(
        proc_macro2::Span::call_site(),
        format!("Failed to read file {}: {}", file_path, e),
      )
      .to_compile_error()
      .into();
    }
  };

  let toml_data = match toml::from_str(&toml_content) {
    Ok(data) => {
      let data: HashMap<String, u32> = data;
      let mut data: Vec<(String, u32)> = data.into_iter().collect();
      data.sort_by_key(|(_, id)| *id);
      data
    }
    Err(e) => {
      return syn::Error::new(
        proc_macro2::Span::call_site(),
        format!("Failed to parse TOML file: {}", e),
      )
      .to_compile_error()
      .into();
    }
  };

  let input_ast = parse_macro_input!(input as DeriveInput);
  let enum_name = &input_ast.ident;

  // 检查是否是枚举
  match input_ast.data {
    syn::Data::Enum(_) => {}
    _ => {
      return TokenStream::from(quote! {
          compile_error!("This macro can only be used on enums");
      });
    }
  }

  let pairs: Vec<_> = toml_data
    .into_iter()
    .map(|(name, id)| {
      let ident = Ident::new(&to_camel_case(&name), Span::call_site());
      (ident, id, name)
    })
    .collect();

  let enum_vars = pairs.iter().map(|(ident, _, _)| {
    quote! {
      #ident
    }
  });

  let vars_id = pairs.iter().map(|(ident, id, _)| {
    quote! {
      #id => #enum_name::#ident
    }
  });

  let label_name = pairs.iter().map(|(ident, _, name)| {
    quote! {
      #enum_name::#ident => String::from(#name)
    }
  });

  let label_id = pairs.iter().map(|(ident, id, _)| {
    quote! {
      #enum_name::#ident => #id
    }
  });

  let vis = &input_ast.vis;

  let label_num = pairs.len() as u32;

  let expanded = quote! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #vis enum #enum_name {
        #(#enum_vars,)*
        Unknown(u32),
    }

    impl WithLabel for #enum_name {
      const LABEL_NUM: u32 = #label_num;
      fn from_label_id(label_id: u32) -> Self {
        match label_id {
          #(#vars_id,)*
          i => #enum_name::Unknown(i),
        }
      }
      fn to_label_str(&self) -> String {
        match self {
          #(#label_name,)*
          #enum_name::Unknown(i) => format!("unknown{}", i),
        }
      }
      fn to_label_id(&self) -> u32 {
        match self {
          #(#label_id, )*
          #enum_name::Unknown(i) => *i,
        }
      }
    }
  };

  // 看这里
  // 如果，你在看这份代码时，根本不知道代码在干啥，或者需要调试，
  // 把下面这行代码取消注释，你就知道这份代生成了一个啥东西了
  // println!("Generated enum:\n{}", expanded);
  // 然后你就可以把生成的代码复制出来，放到 rust playground 里运行看看效果

  TokenStream::from(expanded)
}

fn to_camel_case(s: &str) -> String {
  s.split(' ')
    .map(|word| {
      let mut c = word.chars();
      match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
      }
    })
    .collect()
}
