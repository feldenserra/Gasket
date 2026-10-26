//! Discovers `src/api` and `src/ui` folders.
//!
//! Application authors do not call this crate. `gasket_ui::app!` does.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use std::fs;
use std::path::Path;

#[derive(Debug)]
struct Layout {
    routes: Vec<String>,
    pages: Vec<String>,
}

/// Expand into modules, route glue, and `fn main`.
///
/// The manifest directory is the crate being compiled. Cargo sets that
/// for the rustc process which runs this macro.
#[proc_macro]
pub fn app(input: TokenStream) -> TokenStream {
    let _ = input;
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    match expand(&manifest_dir) {
        Ok(tokens) => tokens.into(),
        Err(message) => syn::Error::new(proc_macro2::Span::call_site(), message)
            .to_compile_error()
            .into(),
    }
}

fn expand(manifest_dir: &str) -> Result<TokenStream2, String> {
    let layout = discover(&Path::new(manifest_dir).join("src"))?;
    Ok(expand_layout(&layout))
}

fn discover(src: &Path) -> Result<Layout, String> {
    let routes = modules(&src.join("api"), "src/api")?;
    let pages = modules(&src.join("ui"), "src/ui")?;
    for page in &pages {
        if !routes.iter().any(|route| route == page) {
            return Err(format!(
                "src/ui/{page} has no matching src/api/{page} route"
            ));
        }
    }
    Ok(Layout { routes, pages })
}

fn modules(dir: &Path, label: &str) -> Result<Vec<String>, String> {
    let entries =
        fs::read_dir(dir).map_err(|_| format!("missing {label} (looked in {})", dir.display()))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| err.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if !path.is_dir() {
            return Err(format!("{label}/{name} must be a folder with mod.rs"));
        }
        if !path.join("mod.rs").is_file() {
            return Err(format!("{label}/{name} is missing mod.rs"));
        }
        if syn::parse_str::<syn::Ident>(&name).is_err() {
            return Err(format!(
                "{label}/{name} must be a Rust name such as hello, not a hyphenated word"
            ));
        }
        names.push(name);
    }
    names.sort();
    Ok(names)
}

fn expand_layout(layout: &Layout) -> TokenStream2 {
    let api_mods = layout.routes.iter().map(|name| {
        let ident = format_ident!("{name}");
        quote! { pub mod #ident; }
    });
    let ui_mods = layout.pages.iter().map(|name| {
        let ident = format_ident!("{name}");
        quote! { pub mod #ident; }
    });
    let calls = layout.routes.iter().map(|name| {
        let ident = format_ident!("call_{name}");
        let module = format_ident!("{name}");
        let message = format!("api/{name} must serialize");
        quote! {
            pub fn #ident() -> gasket_ui::serde_json::Value {
                gasket_ui::serde_json::to_value(super::api::#module::get()).expect(#message)
            }
        }
    });
    let paints = layout.pages.iter().map(|name| {
        let ident = format_ident!("paint_{name}");
        let module = format_ident!("{name}");
        quote! {
            pub fn #ident(ui: &mut gasket_ui::egui::Ui, bytes: &[u8]) {
                match gasket_ui::serde_json::from_slice::<super::api::#module::Response>(bytes) {
                    Ok(data) => super::ui::#module::show(ui, &data),
                    Err(err) => {
                        ui.label(err.to_string());
                    }
                }
            }
        }
    });
    let route_values = layout.routes.iter().map(|name| {
        let path = format!("/api/{name}");
        let call = format_ident!("call_{name}");
        quote! {
            gasket_ui::Route {
                path: #path,
                call: __gasket::#call,
            }
        }
    });
    let page_values = layout.pages.iter().map(|name| {
        let path = format!("/api/{name}");
        let paint = format_ident!("paint_{name}");
        quote! {
            gasket_ui::Page {
                name: #name,
                path: #path,
                paint: __gasket::#paint,
            }
        }
    });

    let api_mod = if layout.routes.is_empty() {
        quote! {}
    } else {
        quote! {
            mod api {
                #(#api_mods)*
            }
        }
    };
    let ui_mod = if layout.pages.is_empty() {
        quote! {}
    } else {
        quote! {
            mod ui {
                #(#ui_mods)*
            }
        }
    };

    quote! {
        #api_mod
        #ui_mod

        mod __gasket {
            #(#calls)*
            #(#paints)*
        }

        fn main() {
            let routes = vec![#(#route_values),*];
            let pages = vec![#(#page_values),*];
            if let Err(err) = gasket_ui::launch(routes, pages) {
                eprintln!("gasket: {err}");
                std::process::exit(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_pairs_pages_with_routes() {
        let root = std::env::temp_dir().join(format!("gasket-macros-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src/api/hello")).unwrap();
        fs::write(root.join("src/api/hello/mod.rs"), b"").unwrap();
        fs::create_dir_all(root.join("src/ui/hello")).unwrap();
        fs::write(root.join("src/ui/hello/mod.rs"), b"").unwrap();

        let layout = discover(&root.join("src")).unwrap();
        assert_eq!(layout.routes, vec!["hello".to_string()]);
        assert_eq!(layout.pages, vec!["hello".to_string()]);
        let tokens = expand_layout(&layout).to_string();
        assert!(tokens.contains("pub mod hello"));
        assert!(tokens.contains("\"/api/hello\""));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn page_without_a_route_is_rejected() {
        let root =
            std::env::temp_dir().join(format!("gasket-macros-missing-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src/api")).unwrap();
        fs::create_dir_all(root.join("src/ui/home")).unwrap();
        fs::write(root.join("src/ui/home/mod.rs"), b"").unwrap();

        let error = discover(&root.join("src")).unwrap_err();
        assert!(error.contains("src/ui/home"));

        fs::remove_dir_all(&root).unwrap();
    }
}
