//! Discovers `src/api` and `src/ui` folders.
//!
//! Application authors do not call this crate. `gasket_ui::app!` does.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use std::fs;
use std::path::Path;

#[derive(Debug)]
struct RouteDecl {
    name: String,
    stream: bool,
    post: bool,
}

#[derive(Debug)]
struct Layout {
    routes: Vec<RouteDecl>,
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
    let routes: Vec<RouteDecl> = modules(&src.join("api"), "src/api")?
        .into_iter()
        .map(|name| {
            let file = src.join("api").join(&name).join("mod.rs");
            RouteDecl {
                name,
                stream: declares_stream(&file),
                post: declares_fn(&file, "post"),
            }
        })
        .collect();
    let pages = modules(&src.join("ui"), "src/ui")?;
    for page in &pages {
        if !routes.iter().any(|route| route.name == *page) {
            return Err(format!(
                "src/ui/{page} has no matching src/api/{page} route"
            ));
        }
    }
    Ok(Layout { routes, pages })
}

/// A route streams when its module defines `STREAM_EVERY`.
fn declares_stream(path: &Path) -> bool {
    source_has(path, "const STREAM_EVERY")
}

/// A route accepts POST when its module defines `fn post(`.
fn declares_fn(path: &Path, name: &str) -> bool {
    source_has(path, &format!("fn {name}("))
}

fn source_has(path: &Path, needle: &str) -> bool {
    let Ok(source) = fs::read_to_string(path) else {
        return false;
    };
    source.lines().any(|line| {
        let code = line.split("//").next().unwrap_or("");
        code.contains(needle)
    })
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
    let api_mods = layout.routes.iter().map(|route| {
        let ident = format_ident!("{}", route.name);
        quote! { pub mod #ident; }
    });
    let ui_mods = layout.pages.iter().map(|name| {
        let ident = format_ident!("{name}");
        quote! { pub mod #ident; }
    });
    let calls = layout.routes.iter().map(|route| {
        let ident = format_ident!("call_{}", route.name);
        let module = format_ident!("{}", route.name);
        let message = format!("api/{} must serialize", route.name);
        quote! {
            pub fn #ident(req: &gasket_ui::Request) -> gasket_ui::serde_json::Value {
                gasket_ui::serde_json::to_value(super::api::#module::get(req)).expect(#message)
            }
        }
    });
    let post_calls = layout
        .routes
        .iter()
        .filter(|route| route.post)
        .map(|route| {
            let ident = format_ident!("post_{}", route.name);
            let module = format_ident!("{}", route.name);
            let message = format!("api/{} post must serialize", route.name);
            quote! {
                pub fn #ident(req: &gasket_ui::Request) -> gasket_ui::serde_json::Value {
                    gasket_ui::serde_json::to_value(super::api::#module::post(req)).expect(#message)
                }
            }
        });
    let paints = layout.pages.iter().map(|name| {
        let ident = format_ident!("paint_{name}");
        let module = format_ident!("{name}");
        quote! {
            pub fn #ident(ui: &mut gasket_ui::egui::Ui, bytes: &[u8], nav: &mut gasket_ui::Nav) {
                match gasket_ui::serde_json::from_slice::<super::api::#module::Response>(bytes) {
                    Ok(data) => super::ui::#module::show(ui, &data, nav),
                    Err(err) => {
                        ui.label(err.to_string());
                    }
                }
            }
        }
    });
    let route_values = layout.routes.iter().map(|route| {
        let path = format!("/api/{}", route.name);
        let call = format_ident!("call_{}", route.name);
        let module = format_ident!("{}", route.name);
        let every = if route.stream {
            quote! { Some(api::#module::STREAM_EVERY) }
        } else {
            quote! { None }
        };
        let post = if route.post {
            let post_fn = format_ident!("post_{}", route.name);
            quote! { Some(__gasket::#post_fn) }
        } else {
            quote! { None }
        };
        quote! {
            gasket_ui::Route {
                path: #path,
                get: __gasket::#call,
                post: #post,
                stream_every: #every,
            }
        }
    });
    let page_values = layout.pages.iter().map(|name| {
        let path = format!("/api/{name}");
        let paint = format_ident!("paint_{name}");
        let module = format_ident!("{name}");
        let every = if layout
            .routes
            .iter()
            .any(|route| route.name == *name && route.stream)
        {
            quote! { Some(api::#module::STREAM_EVERY) }
        } else {
            quote! { None }
        };
        quote! {
            gasket_ui::Page {
                name: #name,
                path: #path,
                stream_every: #every,
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
            #(#post_calls)*
            #(#paints)*
        }

        fn main() {
            let embedded = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/gasket.toml"));
            let routes = vec![#(#route_values),*];
            let pages = vec![#(#page_values),*];
            if let Err(err) = gasket_ui::launch(embedded, routes, pages) {
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
        assert_eq!(layout.routes.len(), 1);
        assert_eq!(layout.routes[0].name, "hello");
        assert!(!layout.routes[0].stream);
        assert_eq!(layout.pages, vec!["hello".to_string()]);
        let tokens = expand_layout(&layout).to_string();
        assert!(tokens.contains("pub mod hello"));
        assert!(tokens.contains("\"/api/hello\""));
        assert!(tokens.contains("stream_every"));

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

    #[test]
    fn stream_every_marks_the_route() {
        let root =
            std::env::temp_dir().join(format!("gasket-macros-stream-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src/api/orders")).unwrap();
        fs::write(
            root.join("src/api/orders/mod.rs"),
            b"pub const STREAM_EVERY: std::time::Duration = std::time::Duration::from_secs(5);\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("src/ui/orders")).unwrap();
        fs::write(root.join("src/ui/orders/mod.rs"), b"").unwrap();

        let layout = discover(&root.join("src")).unwrap();
        assert!(layout.routes[0].stream);
        let tokens = expand_layout(&layout).to_string();
        assert!(tokens.contains("STREAM_EVERY"));

        fs::remove_dir_all(&root).unwrap();
    }
}
