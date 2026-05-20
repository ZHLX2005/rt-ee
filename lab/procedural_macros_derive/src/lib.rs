// 过程宏 crate：编译期 AST 变换引擎
//
// 关键约束：proc-macro = true 的 crate 必须独立，不能导出普通 Rust 代码
// 原因：过程宏在编译期执行，运行环境与普通代码完全不同

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

/// #[derive(Builder)] 的实现
///
/// 输入：
/// #[derive(Builder)]
/// struct User {
///     name: String,
///     age: u32,
/// }
///
/// 输出（编译期生成）：
/// struct UserBuilder {
///     name: Option<String>,
///     age: Option<u32>,
/// }
///
/// impl UserBuilder {
///     fn new() -> Self { ... }
///     fn name(mut self, value: String) -> Self { ... }
///     fn age(mut self, value: u32) -> Self { ... }
///     fn build(self) -> User { ... }
/// }
#[proc_macro_derive(Builder)]
pub fn derive_builder(input: TokenStream) -> TokenStream {
    // Step 1: 将 TokenStream 解析为 AST（抽象语法树）
    let input = parse_macro_input!(input as DeriveInput);

    // Step 2: 提取 struct 名称
    let struct_name = input.ident;
    let builder_name = quote::format_ident!("{}Builder", struct_name);

    // Step 3: 提取命名字段（只支持 named struct）
    let fields = match input.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,
            _ => panic!("Builder derive only supports named fields"),
        },
        _ => panic!("Builder derive only supports structs"),
    };

    // Step 4: 为每个字段生成 Builder 的 Option<T> 字段声明
    let builder_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        quote! { #name: Option<#ty> }
    });

    // Step 5: 为 new() 生成初始化代码（所有字段为 None）
    let none_fields = fields.iter().map(|f| {
        let name = &f.ident;
        quote! { #name: None }
    });

    // Step 6: 为每个字段生成 setter 方法
    let setters = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        quote! {
            pub fn #name(mut self, value: #ty) -> Self {
                self.#name = Some(value);
                self
            }
        }
    });

    // Step 7: 为 build() 生成字段赋值（unwrap Option）
    let build_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let name_str = name.as_ref().unwrap().to_string();
        quote! {
            #name: self.#name.expect(concat!("Builder missing field: ", #name_str))
        }
    });

    // Step 8: 用 quote! 生成新的 TokenStream
    let expanded = quote! {
        pub struct #builder_name {
            #(#builder_fields,)*
        }

        impl #builder_name {
            pub fn new() -> Self {
                Self {
                    #(#none_fields,)*
                }
            }

            #(#setters)*

            pub fn build(self) -> #struct_name {
                #struct_name {
                    #(#build_fields,)*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// 属性宏示例：#[trace_function]
/// 为函数添加进入/退出日志
#[proc_macro_attribute]
pub fn trace_function(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // 解析输入为函数
    let input = parse_macro_input!(item as syn::ItemFn);
    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let fn_sig = &input.sig;
    let fn_block = &input.block;
    let fn_attrs = &input.attrs;

    let fn_name_str = fn_name.to_string();

    let expanded = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            println!("[TRACE] Entering: {}", #fn_name_str);
            let __result = #fn_block;
            println!("[TRACE] Exiting: {}", #fn_name_str);
            __result
        }
    };

    TokenStream::from(expanded)
}
