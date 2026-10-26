//! Native shell for Gasket applications.
//!
//! An application is [`app`]: `src/api/<name>` is a route and `src/ui/<name>`
//! is the page that draws that route's `Response`. This crate starts the
//! server, opens the window, and fetches the route. egui is the placeholder
//! page UI until stitch replaces it.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderMap, Method, Uri};
use axum::routing::get;
use axum::{Json, Router};

#[doc(hidden)]
pub use gasket_macros::app as app_impl;
#[doc(hidden)]
pub use serde_json;

/// Placeholder page UI. Pages draw into this until stitch exists.
pub use eframe::egui;

/// Discover `src/api` routes and `src/ui` pages, then start the app.
///
/// Put this alone in `main.rs`. `src/api/<name>/mod.rs` defines `get() -> Response`.
/// `src/ui/<name>/mod.rs` defines `show(ui, data, nav)` for that same response.
/// `nav.open("name")` switches pages. A page named `index` is the home
/// dashboard. A route that defines `STREAM_EVERY` is pushed on `/ws` at that
/// interval while that page is visible. The shell keeps one socket and
/// closes it when you leave. `get` runs for the first paint and for each
/// push. The page still only draws the response.
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
    pub get: fn(&Request) -> serde_json::Value,
    pub post: Option<fn(&Request) -> serde_json::Value>,
    /// Set from the route's `STREAM_EVERY`. `None` is one HTTP response.
    pub stream_every: Option<Duration>,
}

const BODY_LIMIT: usize = 1024 * 1024;

/// The HTTP request a route handler receives.
pub struct Request {
    pub method: String,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    /// A GET with no query, headers, or body. Stream pushes use this.
    pub fn get(path: impl Into<String>) -> Self {
        Self {
            method: "GET".into(),
            path: path.into(),
            query: Vec::new(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    /// The first query value named `name`.
    pub fn query(&self, name: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// The first header named `name`, compared without case.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Parse the body as JSON.
    ///
    /// # Errors
    ///
    /// Returns the serde error when the body is not that type.
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.body)
    }
}

/// Listen address and the URL the window dials.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub client: ClientConfig,
}

/// Where `--serve` and the default mode listen.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct ServerConfig {
    pub bind: String,
    pub port: u16,
}

/// Where `--ui` and the default mode open pages.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct ClientConfig {
    pub url: String,
}

impl Config {
    /// Parse `gasket.toml` text.
    ///
    /// # Errors
    ///
    /// Returns an error when the file is not the expected tables.
    pub fn parse(text: &str) -> Result<Self, String> {
        toml::from_str(text).map_err(|err| err.to_string())
    }

    /// `./gasket.toml`, then a file beside the executable, then `embedded`.
    ///
    /// # Errors
    ///
    /// Returns an error when the chosen file does not parse.
    pub fn load(embedded: &str) -> Result<Self, String> {
        if let Ok(text) = std::fs::read_to_string("gasket.toml") {
            return Self::parse(&text);
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let path = dir.join("gasket.toml");
                if let Ok(text) = std::fs::read_to_string(path) {
                    return Self::parse(&text);
                }
            }
        }
        Self::parse(embedded)
    }

    fn listen(&self) -> Result<std::net::SocketAddr, String> {
        let ip: std::net::IpAddr = self
            .server
            .bind
            .parse()
            .map_err(|_| format!("invalid bind address: {}", self.server.bind))?;
        Ok(std::net::SocketAddr::new(ip, self.server.port))
    }
}

enum RunMode {
    Both,
    Serve,
    Ui,
}

/// One page discovered from `src/ui`, fed by the route of the same name.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct Page {
    pub name: &'static str,
    pub path: &'static str,
    /// Copied from the paired route. The page function does not see it.
    pub stream_every: Option<Duration>,
    pub paint: fn(&mut egui::Ui, &[u8], &mut Nav<'_>),
}

/// Lets a page open another page, keep a text field, or POST JSON to its route.
pub struct Nav<'a> {
    page: &'static str,
    names: &'a [&'static str],
    target: &'a mut Option<usize>,
    fields: &'a mut HashMap<String, String>,
    post: &'a mut Option<Vec<u8>>,
}

impl Nav<'_> {
    /// Open the page whose folder is `page`. An unknown name does nothing.
    pub fn open(&mut self, page: &str) {
        if let Some(index) = self.names.iter().position(|name| *name == page) {
            *self.target = Some(index);
        }
    }

    /// A text value that survives redraws. Use with `ui.text_edit_singleline`.
    pub fn field(&mut self, name: &str) -> &mut String {
        self.fields
            .entry(format!("{}:{name}", self.page))
            .or_default()
    }

    /// POST `body` to this page's route. The next `show` receives the reply.
    pub fn post(&mut self, body: serde_json::Value) {
        *self.post = Some(body.to_string().into_bytes());
    }
}

/// Start core, then open the shell on this thread.
///
/// # Errors
///
/// Returns an error when the port argument is invalid, the listener cannot
/// bind, or the window fails to open.
#[doc(hidden)]
pub fn launch(
    embedded: &str,
    routes: Vec<Route>,
    pages: Vec<Page>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mode = run_mode()?;
    let config = Config::load(embedded)?;
    match mode {
        RunMode::Serve => {
            let addr = start_core(config.listen()?, routes)?;
            println!("Gasket core listening on http://{addr}");
            std::thread::park();
            Ok(())
        }
        RunMode::Ui => {
            println!("Gasket window using {}", config.client.url);
            run_shell(config.client.url, pages)?;
            Ok(())
        }
        RunMode::Both => {
            let addr = start_core(config.listen()?, routes)?;
            println!("Gasket core listening on http://{addr}");
            run_shell(config.client.url, pages)?;
            Ok(())
        }
    }
}

fn run_mode() -> Result<RunMode, String> {
    let mut serve = false;
    let mut ui = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--serve" => serve = true,
            "--ui" => ui = true,
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    match (serve, ui) {
        (true, true) => Err("use only one of --serve or --ui".into()),
        (true, false) => Ok(RunMode::Serve),
        (false, true) => Ok(RunMode::Ui),
        (false, false) => Ok(RunMode::Both),
    }
}

fn start_core(
    addr: SocketAddr,
    routes: Vec<Route>,
) -> Result<SocketAddr, Box<dyn std::error::Error>> {
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
            let listener = match gasket_core::bind(addr).await {
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
            if let Err(err) = gasket_core::serve(listener, app).await {
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

fn build_router(routes: &[Route]) -> Router {
    let shared: Arc<[Route]> = routes.to_vec().into();
    let router = shared.iter().fold(Router::new(), |router, route| {
        let path = route.path;
        let get_fn = route.get;
        let post_fn = route.post;
        if let Some(post_fn) = post_fn {
            router.route(
                path,
                get(move |method, uri, headers, body| {
                    call_route(method, uri, headers, body, get_fn)
                })
                .post(move |method, uri, headers, body| {
                    call_route(method, uri, headers, body, post_fn)
                }),
            )
        } else {
            router.route(
                path,
                get(move |method, uri, headers, body| {
                    call_route(method, uri, headers, body, get_fn)
                }),
            )
        }
    });
    let ws_routes = Arc::clone(&shared);
    router
        .route(
            "/ws",
            get(move |upgrade: WebSocketUpgrade| {
                let ws_routes = Arc::clone(&ws_routes);
                async move { upgrade.on_upgrade(move |socket| push_route(socket, ws_routes)) }
            }),
        )
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
}

async fn call_route(
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
    call: fn(&Request) -> serde_json::Value,
) -> Json<serde_json::Value> {
    Json(call(&request_from_http(method, &uri, &headers, &body)))
}

fn request_from_http(method: Method, uri: &Uri, headers: &HeaderMap, body: &[u8]) -> Request {
    let query = uri.query().map(parse_query).unwrap_or_default();
    let headers = headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.to_string(), value.to_string()))
        })
        .collect();
    Request {
        method: method.to_string(),
        path: uri.path().to_string(),
        query,
        headers,
        body: body.to_vec(),
    }
}

fn parse_query(raw: &str) -> Vec<(String, String)> {
    raw.split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (decode_component(key), decode_component(value))
        })
        .collect()
}

fn decode_component(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let hex = &raw[index + 1..index + 3];
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    index += 3;
                } else {
                    out.push(b'%');
                    index += 1;
                }
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The client sends one text frame, the route path. The server replies with
/// that route's JSON. If the route set `STREAM_EVERY`, it keeps sending at
/// that interval until the socket closes.
async fn push_route(mut socket: WebSocket, routes: Arc<[Route]>) {
    let Some(Ok(Message::Text(text))) = socket.recv().await else {
        return;
    };
    let path = text.trim();
    let Some(route) = routes.iter().find(|route| route.path == path) else {
        let _ = socket
            .send(Message::Text(format!(
                r#"{{"error":"unknown route {path}"}}"#
            )))
            .await;
        return;
    };
    let call = route.get;
    let every = route.stream_every.filter(|every| !every.is_zero());
    let request = Request::get(path.to_string());
    loop {
        let json = serde_json::to_string(&call(&request)).unwrap_or_else(|_| "{}".to_string());
        if socket.send(Message::Text(json)).await.is_err() {
            break;
        }
        let Some(every) = every else {
            break;
        };
        // A closed socket must wake this wait. Calling `get()` after the
        // client has left would move the route's counter while the page is gone.
        tokio::select! {
            () = tokio::time::sleep(every) => {}
            _ = socket.recv() => break,
        }
    }
}

fn run_shell(url: String, pages: Vec<Page>) -> Result<(), Box<dyn std::error::Error>> {
    let authority = authority_from_url(&url)?;
    let viewport = egui::ViewportBuilder::default().with_inner_size([480.0, 280.0]);
    eframe::run_native(
        "Gasket",
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(move |_ctx| Ok(Box::new(ShellApp::new(url, authority, pages)))),
    )?;
    Ok(())
}

fn authority_from_url(url: &str) -> Result<String, String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or("client.url must start with http://")?;
    let authority = rest.split('/').next().unwrap_or("");
    if authority.is_empty() || authority.contains(' ') {
        return Err(format!("invalid client.url: {url}"));
    }
    if authority.contains(':') {
        Ok(authority.to_string())
    } else {
        Ok(format!("{authority}:80"))
    }
}

fn http_get(authority: &str, path: &str) -> Result<Vec<u8>, String> {
    http_request(authority, "GET", path, &[])
}

fn http_request(authority: &str, method: &str, path: &str, body: &[u8]) -> Result<Vec<u8>, String> {
    let mut stream = std::net::TcpStream::connect(authority).map_err(|err| err.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|err| err.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|err| err.to_string())?;
    let extra = if method == "POST" {
        format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            body.len()
        )
    } else {
        String::new()
    };
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\nAccept: application/json\r\n{extra}\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|err| err.to_string())?;
    if !body.is_empty() {
        stream.write_all(body).map_err(|err| err.to_string())?;
    }

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
    url: String,
    authority: String,
    pages: Vec<Page>,
    current: usize,
    body: Option<Vec<u8>>,
    error: Option<String>,
    pending: Option<mpsc::Receiver<Result<Vec<u8>, String>>>,
    live: bool,
    stop_stream: Option<Arc<AtomicBool>>,
    fields: HashMap<String, String>,
}

impl ShellApp {
    fn new(url: String, authority: String, pages: Vec<Page>) -> Self {
        let current = pages
            .iter()
            .position(|page| page.name == "index")
            .unwrap_or(0);
        let mut app = Self {
            url,
            authority,
            pages,
            current,
            body: None,
            error: None,
            pending: None,
            live: false,
            stop_stream: None,
            fields: HashMap::new(),
        };
        app.refresh();
        app
    }

    fn go(&mut self, index: usize) {
        self.current = index;
        self.body = None;
        self.error = None;
        self.refresh();
    }

    fn refresh(&mut self) {
        if let Some(stop) = self.stop_stream.take() {
            stop.store(true, Ordering::Relaxed);
        }
        self.pending = None;
        let Some(page) = self.pages.get(self.current).copied() else {
            self.live = false;
            return;
        };
        self.error = None;
        self.live = page.stream_every.is_some();
        if self.live {
            self.watch(page.path);
        } else {
            self.fetch(page.path);
        }
    }

    fn fetch(&mut self, path: &'static str) {
        let authority = self.authority.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(http_get(&authority, path));
        });
        self.pending = Some(rx);
    }

    fn submit(&mut self, path: &'static str, body: Vec<u8>) {
        if self.pending.is_some() {
            return;
        }
        let authority = self.authority.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(http_request(&authority, "POST", path, &body));
        });
        self.pending = Some(rx);
    }

    fn watch(&mut self, path: &'static str) {
        let authority = self.authority.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let flag = Arc::clone(&stop);
        std::thread::spawn(move || watch_ws(&authority, path, flag, tx));
        self.stop_stream = Some(stop);
        self.pending = Some(rx);
    }

    fn poll(&mut self) {
        let mut done = false;
        if let Some(rx) = &self.pending {
            loop {
                match rx.try_recv() {
                    Ok(Ok(body)) => {
                        self.body = Some(body);
                        self.error = None;
                        if !self.live {
                            done = true;
                            break;
                        }
                    }
                    Ok(Err(err)) => {
                        self.error = Some(err);
                        done = true;
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.error = Some("Route request dropped.".into());
                        done = true;
                        break;
                    }
                }
            }
        }
        if done {
            self.pending = None;
            self.live = false;
        }
    }
}

impl eframe::App for ShellApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
        let names: Vec<&'static str> = self.pages.iter().map(|page| page.name).collect();
        let has_index = names.contains(&"index");
        let current_name = names.get(self.current).copied();
        let page = self.pages.get(self.current).copied();
        let body = self.body.clone();
        let error = self.error.clone();
        let waiting = self.pending.is_some() && body.is_none() && error.is_none();
        let live = self.live;
        let url = self.url.clone();
        let authority = self.authority.clone();
        let mut destination = None;
        let mut refresh = false;
        let mut posted = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            if has_index && current_name != Some("index") && ui.button("Back").clicked() {
                destination = names.iter().position(|name| *name == "index");
            }
            if !has_index && names.len() > 1 {
                ui.horizontal(|ui| {
                    for (index, name) in names.iter().enumerate() {
                        if ui.selectable_label(index == self.current, *name).clicked() {
                            destination = Some(index);
                        }
                    }
                });
            }
            ui.label(format!("Core: {url}"));
            if live {
                ui.label(format!("Live: ws://{authority}/ws"));
            }
            ui.separator();

            if waiting {
                ui.label("Loading...");
            } else if let Some(err) = &error {
                ui.label(err);
            } else if let (Some(page), Some(body)) = (page, body.as_deref()) {
                let mut nav = Nav {
                    page: page.name,
                    names: &names,
                    target: &mut destination,
                    fields: &mut self.fields,
                    post: &mut posted,
                };
                (page.paint)(ui, body, &mut nav);
            } else if names.is_empty() {
                ui.label("This app has no pages.");
            }

            ui.add_space(10.0);
            if !live && !names.is_empty() && ui.button("Refresh").clicked() {
                refresh = true;
            }
        });

        if self.live || self.pending.is_some() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        if let Some(index) = destination {
            if index != self.current {
                self.go(index);
            }
        } else if let Some(body) = posted {
            if let Some(page) = self.pages.get(self.current) {
                self.submit(page.path, body);
            }
        } else if refresh {
            self.refresh();
        }
    }
}

fn watch_ws(
    authority: &str,
    path: &str,
    stop: Arc<AtomicBool>,
    tx: mpsc::Sender<Result<Vec<u8>, String>>,
) {
    let url = format!("ws://{authority}/ws");
    let mut socket = match tungstenite::connect(&url) {
        Ok((socket, _)) => socket,
        Err(err) => {
            let _ = tx.send(Err(err.to_string()));
            return;
        }
    };
    if socket
        .send(tungstenite::Message::Text(path.to_string()))
        .is_err()
    {
        let _ = tx.send(Err("could not subscribe on /ws".into()));
        return;
    }
    if let tungstenite::stream::MaybeTlsStream::Plain(tcp) = socket.get_mut() {
        // A blocking read would sit until the next push, so leaving the page
        // could not close the socket until the server had already called `get()`.
        let _ = tcp.set_nonblocking(true);
    }
    while !stop.load(Ordering::Relaxed) {
        match socket.read() {
            Ok(tungstenite::Message::Text(text)) => {
                if tx.send(Ok(text.into_bytes())).is_err() {
                    break;
                }
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(err))
                if err.kind() == std::io::ErrorKind::WouldBlock
                    || err.kind() == std::io::ErrorKind::TimedOut =>
            {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(err) => {
                let _ = tx.send(Err(err.to_string()));
                break;
            }
        }
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
            get: |_req| {
                serde_json::json!({
                    "message": "Hello from the Gasket reference app",
                    "status": "ok"
                })
            },
            post: None,
            stream_every: None,
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
                let listener = gasket_core::bind(std::net::SocketAddr::from(([127, 0, 0, 1], 0)))
                    .await
                    .unwrap();
                ready_tx.send(listener.local_addr().unwrap()).unwrap();
                gasket_core::serve(listener, build_router(&routes))
                    .await
                    .unwrap();
            });
        });
        let addr = ready_rx.recv().unwrap();

        let body = http_get(&addr.to_string(), "/api/hello").unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["message"], "Hello from the Gasket reference app");
        assert_eq!(value["status"], "ok");
    }

    #[test]
    fn websocket_pushes_the_route() {
        let routes = vec![hello_route()];
        let (ready_tx, ready_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let listener = gasket_core::bind(std::net::SocketAddr::from(([127, 0, 0, 1], 0)))
                    .await
                    .unwrap();
                ready_tx.send(listener.local_addr().unwrap()).unwrap();
                gasket_core::serve(listener, build_router(&routes))
                    .await
                    .unwrap();
            });
        });
        let addr = ready_rx.recv().unwrap();
        let (mut socket, _) = tungstenite::connect(format!("ws://{addr}/ws")).unwrap();
        socket
            .send(tungstenite::Message::Text("/api/hello".into()))
            .unwrap();
        let frame = socket.read().unwrap();
        let tungstenite::Message::Text(text) = frame else {
            panic!("expected a text frame, got {frame:?}");
        };
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["message"], "Hello from the Gasket reference app");
        assert_eq!(value["status"], "ok");
    }

    #[test]
    fn websocket_waits_for_the_route_interval() {
        let routes = vec![Route {
            path: "/api/orders",
            get: |_req| serde_json::json!({"ok": true}),
            post: None,
            stream_every: Some(Duration::from_millis(80)),
        }];
        let (ready_tx, ready_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let listener = gasket_core::bind(std::net::SocketAddr::from(([127, 0, 0, 1], 0)))
                    .await
                    .unwrap();
                ready_tx.send(listener.local_addr().unwrap()).unwrap();
                gasket_core::serve(listener, build_router(&routes))
                    .await
                    .unwrap();
            });
        });
        let addr = ready_rx.recv().unwrap();
        let (mut socket, _) = tungstenite::connect(format!("ws://{addr}/ws")).unwrap();
        socket
            .send(tungstenite::Message::Text("/api/orders".into()))
            .unwrap();
        let _first = socket.read().unwrap();
        let started = std::time::Instant::now();
        let _second = socket.read().unwrap();
        let elapsed = started.elapsed();
        assert!(
            elapsed >= Duration::from_millis(50),
            "interval was too short: {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_millis(500),
            "interval ignored the route and waited {elapsed:?}"
        );
    }

    #[test]
    fn config_parses_bind_and_client_url() {
        let text = "[server]\nbind = \"0.0.0.0\"\nport = 8080\n\n[client]\nurl = \"http://example.com:8080\"\n";
        let config = Config::parse(text).unwrap();
        assert_eq!(config.server.bind, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.client.url, "http://example.com:8080");
        assert_eq!(
            config.listen().unwrap(),
            "0.0.0.0:8080".parse::<std::net::SocketAddr>().unwrap()
        );
        assert!(Config::parse("bind = \"nope\"").is_err());
    }

    #[tokio::test]
    async fn get_reads_query_and_post_reads_json() {
        let route = Route {
            path: "/api/echo",
            get: |req| serde_json::json!({ "name": req.query("name") }),
            post: Some(|req| {
                req.json::<serde_json::Value>()
                    .unwrap_or_else(|_| serde_json::json!({"status": "bad json"}))
            }),
            stream_every: None,
        };
        let app = build_router(&[route]);
        let get_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/echo?name=Ada")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let get_bytes = axum::body::to_bytes(get_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let get_value: serde_json::Value = serde_json::from_slice(&get_bytes).unwrap();
        assert_eq!(get_value["name"], "Ada");

        let post_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/echo")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"message":"hi"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(post_response.status().is_success());
        let post_bytes = axum::body::to_bytes(post_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let post_value: serde_json::Value = serde_json::from_slice(&post_bytes).unwrap();
        assert_eq!(post_value["message"], "hi");
    }

    #[test]
    fn serve_answers_http_without_a_window() {
        let addr = start_core(
            std::net::SocketAddr::from(([127, 0, 0, 1], 0)),
            vec![hello_route()],
        )
        .unwrap();
        let body = http_get(&addr.to_string(), "/api/hello").unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "ok");
    }
}
