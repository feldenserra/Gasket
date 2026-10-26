//! Native shell for Gasket applications.
//!
//! An application is [`app`]: `src/api/<name>` is a route and `src/ui/<name>`
//! is the page that draws that route's `Response`. This crate starts the
//! server, opens the window, and fetches the route. egui is the placeholder
//! page UI until stitch replaces it.

use std::io::{Read, Write};
use std::net::SocketAddr;
use std::sync::mpsc;
use std::time::Duration;

use axum::routing::get;
use axum::{Json, Router};
use gasket_protocol::{ClientMessage, ServerMessage};

#[doc(hidden)]
pub use gasket_macros::app as app_impl;
#[doc(hidden)]
pub use serde_json;

/// Placeholder page UI. Pages draw into this until stitch exists.
pub use eframe::egui;

/// Discover `src/api` routes and `src/ui` pages, then start the app.
///
/// Put this alone in `main.rs`. `src/api/<name>/mod.rs` defines `get() -> Response`.
/// `src/ui/<name>/mod.rs` defines `show(ui, data)` for that same response.
/// A page with no matching route does not compile. `get` should return quickly;
/// it runs on the server.
#[macro_export]
macro_rules! app {
    () => {
        $crate::app_impl!();
    };
}

/// One `GET /api/<name>` route discovered from `src/api`.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct Route {
    pub path: &'static str,
    pub call: fn() -> serde_json::Value,
}

/// One page discovered from `src/ui`, fed by the route of the same name.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct Page {
    pub name: &'static str,
    pub path: &'static str,
    pub paint: fn(&mut egui::Ui, &[u8]),
}

/// Start core, then open the shell on this thread.
///
/// # Errors
///
/// Returns an error when the port argument is invalid, the listener cannot
/// bind, or the window fails to open.
#[doc(hidden)]
pub fn launch(routes: Vec<Route>, pages: Vec<Page>) -> Result<(), Box<dyn std::error::Error>> {
    let port = port_from_args()?;
    let addr = start_core(port, routes)?;
    println!("Gasket core listening on http://{addr}");
    run_shell(addr, pages)?;
    Ok(())
}

fn port_from_args() -> Result<u16, String> {
    let mut port = 3000u16;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if let Some(value) = arg.strip_prefix("--port=") {
            port = parse_port(value)?;
        } else if arg == "--port" {
            let value = args
                .next()
                .ok_or_else(|| "missing value for --port".to_string())?;
            port = parse_port(&value)?;
        } else {
            return Err(format!("unknown argument: {arg}"));
        }
    }
    Ok(port)
}

fn parse_port(value: &str) -> Result<u16, String> {
    value.parse().map_err(|_| format!("invalid port: {value}"))
}

fn start_core(port: u16, routes: Vec<Route>) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let (ready_tx, ready_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(err) => {
                let _ = ready_tx.send(Err(err.to_string()));
                return;
            }
        };
        runtime.block_on(async move {
            let listener = match gasket_core::bind(port).await {
                Ok(listener) => listener,
                Err(err) => {
                    let _ = ready_tx.send(Err(err.to_string()));
                    return;
                }
            };
            let addr = match listener.local_addr() {
                Ok(addr) => addr,
                Err(err) => {
                    let _ = ready_tx.send(Err(err.to_string()));
                    return;
                }
            };
            let _ = ready_tx.send(Ok(addr));
            let app = build_router(&routes);
            if let Err(err) = gasket_core::serve(listener, app, stream_handler).await {
                eprintln!("gasket: core stopped: {err}");
            }
        });
    });

    match ready_rx.recv() {
        Ok(Ok(addr)) => Ok(addr),
        Ok(Err(err)) => Err(err.into()),
        Err(_) => Err("core thread exited before binding".into()),
    }
}

fn stream_handler(message: ClientMessage) -> ServerMessage {
    match message {
        ClientMessage::RequestHello => ServerMessage::Hello {
            message: "ok".into(),
            status: "ok".into(),
        },
    }
}

fn build_router(routes: &[Route]) -> Router {
    routes.iter().fold(Router::new(), |router, route| {
        let path = route.path;
        let call = route.call;
        router.route(path, get(move || async move { Json(call()) }))
    })
}

fn run_shell(addr: SocketAddr, pages: Vec<Page>) -> eframe::Result<()> {
    let viewport = egui::ViewportBuilder::default().with_inner_size([480.0, 280.0]);
    eframe::run_native(
        "Gasket",
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(move |_ctx| Ok(Box::new(ShellApp::new(addr, pages)))),
    )
}

fn http_get(addr: SocketAddr, path: &str) -> Result<Vec<u8>, String> {
    let mut stream = std::net::TcpStream::connect(addr).map_err(|err| err.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|err| err.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|err| err.to_string())?;
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\nAccept: application/json\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|err| err.to_string())?;

    let mut buffer = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        if let Some(body) = complete_body(&buffer)? {
            return Ok(body);
        }
        match stream.read(&mut chunk) {
            Ok(0) => return finish_response(&buffer),
            Ok(size) => buffer.extend_from_slice(&chunk[..size]),
            Err(err)
                if err.kind() == std::io::ErrorKind::WouldBlock
                    || err.kind() == std::io::ErrorKind::TimedOut =>
            {
                return finish_response(&buffer);
            }
            Err(err) => return Err(err.to_string()),
        }
    }
}

fn finish_response(buffer: &[u8]) -> Result<Vec<u8>, String> {
    if let Some(body) = complete_body(buffer)? {
        return Ok(body);
    }
    let Some(split) = buffer.windows(4).position(|window| window == b"\r\n\r\n") else {
        return Err("incomplete HTTP response".into());
    };
    let head = String::from_utf8_lossy(&buffer[..split]);
    let body = buffer[split + 4..].to_vec();
    status_body(&head, body)
}

fn complete_body(buffer: &[u8]) -> Result<Option<Vec<u8>>, String> {
    let Some(split) = buffer.windows(4).position(|window| window == b"\r\n\r\n") else {
        return Ok(None);
    };
    let head = String::from_utf8_lossy(&buffer[..split]);
    let raw_body = &buffer[split + 4..];
    let length =
        header_value(&head, "content-length").and_then(|value| value.parse::<usize>().ok());
    let body = if let Some(length) = length {
        if raw_body.len() < length {
            return Ok(None);
        }
        raw_body[..length].to_vec()
    } else {
        return Ok(None);
    };
    status_body(&head, body).map(Some)
}

fn status_body(head: &str, body: Vec<u8>) -> Result<Vec<u8>, String> {
    let status = head.lines().next().unwrap_or("");
    if !status.contains(" 200 ") {
        return Err(format!("{status}: {}", String::from_utf8_lossy(&body)));
    }
    Ok(body)
}

fn header_value<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    head.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if key.eq_ignore_ascii_case(name) {
            Some(value.trim())
        } else {
            None
        }
    })
}

struct ShellApp {
    addr: SocketAddr,
    pages: Vec<Page>,
    current: usize,
    body: Option<Vec<u8>>,
    error: Option<String>,
    pending: Option<mpsc::Receiver<Result<Vec<u8>, String>>>,
}

impl ShellApp {
    fn new(addr: SocketAddr, pages: Vec<Page>) -> Self {
        let mut app = Self {
            addr,
            pages,
            current: 0,
            body: None,
            error: None,
            pending: None,
        };
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        let Some(page) = self.pages.get(self.current).copied() else {
            return;
        };
        let addr = self.addr;
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(http_get(addr, page.path));
        });
        self.pending = Some(rx);
        self.error = None;
    }

    fn poll(&mut self) {
        let Some(rx) = &self.pending else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(body)) => {
                self.body = Some(body);
                self.error = None;
                self.pending = None;
            }
            Ok(Err(err)) => {
                self.error = Some(err);
                self.pending = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.error = Some("Route request dropped.".into());
                self.pending = None;
            }
        }
    }
}

impl eframe::App for ShellApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut switch = None;
            if self.pages.len() > 1 {
                ui.horizontal(|ui| {
                    for (index, page) in self.pages.iter().enumerate() {
                        if ui
                            .selectable_label(index == self.current, page.name)
                            .clicked()
                        {
                            switch = Some(index);
                        }
                    }
                });
            }
            if let Some(index) = switch {
                if index != self.current {
                    self.current = index;
                    self.body = None;
                    self.refresh();
                }
            }
            ui.label(format!("Core: http://{}", self.addr));
            ui.separator();

            if self.pending.is_some() && self.body.is_none() && self.error.is_none() {
                ui.label("Loading...");
                ctx.request_repaint_after(Duration::from_millis(50));
            } else if let Some(err) = &self.error {
                ui.label(err);
            } else if let (Some(page), Some(body)) =
                (self.pages.get(self.current), self.body.as_deref())
            {
                (page.paint)(ui, body);
            } else if self.pages.is_empty() {
                ui.label("This app has no pages.");
            }

            if self.pending.is_some() && self.body.is_some() {
                ctx.request_repaint_after(Duration::from_millis(50));
            }

            ui.add_space(10.0);
            if !self.pages.is_empty() && ui.button("Refresh").clicked() {
                self.refresh();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn hello_route() -> Route {
        Route {
            path: "/api/hello",
            call: || {
                serde_json::json!({
                    "message": "Hello from the Gasket reference app",
                    "status": "ok"
                })
            },
        }
    }

    #[tokio::test]
    async fn get_route_returns_json() {
        let response = build_router(&[hello_route()])
            .oneshot(
                Request::builder()
                    .uri("/api/hello")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(response.status().is_success());
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["message"], "Hello from the Gasket reference app");
        assert_eq!(value["status"], "ok");
    }

    #[test]
    fn shell_get_reads_the_route() {
        let routes = vec![hello_route()];
        let (ready_tx, ready_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let listener = gasket_core::bind(0).await.unwrap();
                ready_tx.send(listener.local_addr().unwrap()).unwrap();
                gasket_core::serve(listener, build_router(&routes), stream_handler)
                    .await
                    .unwrap();
            });
        });
        let addr = ready_rx.recv().unwrap();

        let body = http_get(addr, "/api/hello").unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["message"], "Hello from the Gasket reference app");
        assert_eq!(value["status"], "ok");
    }
}
