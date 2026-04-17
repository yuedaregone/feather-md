use crate::config::{Config, WindowState};
use crate::file_association;
use crate::file_watcher::FileWatcher;
use crate::resources::Assets;
use percent_encoding::percent_decode_str;
use rfd::FileDialog;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy},
    window::WindowBuilder,
};
use wry::{
    http::{header::CONTENT_TYPE, Request, Response as HttpResponse},
    WebViewBuilder,
};

/// Custom events for the application
#[derive(Debug, Clone)]
pub enum AppEvent {
    FileChanged,
    OpenFile(PathBuf),
    FrontendReady,
}

pub struct App {
    config: Config,
}

impl App {
    pub fn new() -> Self {
        let config = Config::load();
        Self { config }
    }

    pub fn run(&mut self, file_path: Option<PathBuf>) {
        let event_loop: EventLoop<AppEvent> = EventLoopBuilder::with_user_event().build();
        let event_loop_proxy = event_loop.create_proxy();

        let window = WindowBuilder::new()
            .with_title("FeatherMD")
            .with_inner_size(tao::dpi::LogicalSize::new(
                self.config.window.width,
                self.config.window.height,
            ))
            .build(&event_loop)
            .expect("Failed to create window");

        let file_dir: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(
            file_path
                .as_ref()
                .and_then(|p| p.parent().map(|d| d.to_path_buf())),
        ));

        let current_file: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(file_path.clone()));

        let file_watcher: Arc<Mutex<Option<FileWatcher>>> = Arc::new(Mutex::new(
            if let Some(ref path) = file_path {
                FileWatcher::new(path, event_loop_proxy.clone()).ok()
            } else {
                None
            },
        ));

        let webview_builder = WebViewBuilder::new()
            .with_url("feather://app/index.html")
            .with_initialization_script(INIT_SCRIPT)
            .with_drag_drop_handler({
                let event_loop_proxy = event_loop_proxy.clone();
                move |event| {
                    if let wry::DragDropEvent::Drop { paths, .. } = event {
                        if let Some(path) = paths.first() {
                            let _ = event_loop_proxy.send_event(AppEvent::OpenFile(path.clone()));
                        }
                    }
                    true
                }
            })
            .with_ipc_handler({
                let event_loop_proxy = event_loop_proxy.clone();
                move |request: Request<String>| {
                    handle_ipc(request.body(), &event_loop_proxy);
                }
            })
            .with_custom_protocol("feather".into(), {
                let file_dir = file_dir.clone();
                move |_webview_id, request| handle_custom_protocol(request, &file_dir).map(Into::into)
            });

        #[cfg(any(target_os = "windows", target_os = "macos"))]
        let webview = webview_builder
            .build(&window)
            .expect("Failed to create webview");

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let webview = {
            use tao::platform::unix::WindowExtUnix;
            use wry::WebViewBuilderExtUnix;
            let vbox = window.default_vbox().expect("No vbox");
            webview_builder.build_gtk(vbox).expect("Failed to create webview")
        };
        
        let webview = Arc::new(webview);

        let theme_script = format!(
            "window.__feather_set_theme({})",
            serde_json::to_string(&self.config.theme).unwrap()
        );
        let _ = webview.evaluate_script(&theme_script);

        if !file_association::is_registered() {
            if let Ok(exe_path) = std::env::current_exe() {
                let _ = file_association::register_association(&exe_path.to_string_lossy());
            }
        }

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::UserEvent(app_event) => match app_event {
                    AppEvent::FrontendReady => {
                        if let Some(path) = current_file.lock().unwrap().as_ref() {
                            if let Ok(content) = read_file_with_encoding(path) {
                                let path_str = path.to_string_lossy().to_string();
                                let script = format!(
                                    "window.__feather_render({}, {})",
                                    serde_json::to_string(&content).unwrap(),
                                    serde_json::to_string(&path_str).unwrap()
                                );
                                let _ = webview.evaluate_script(&script);
                            } else {
                                let path_str = path.to_string_lossy().to_string();
                                let script = format!(
                                    "window.__feather_error('无法打开文件', '文件不存在或无法读取: {}')",
                                    serde_json::to_string(&path_str).unwrap()
                                );
                                let _ = webview.evaluate_script(&script);
                            }
                        }
                    }
                    AppEvent::FileChanged => {
                        let file_path_lock = current_file.lock().unwrap();
                        if let Some(ref path) = *file_path_lock {
                            if let Ok(content) = read_file_with_encoding(path) {
                                let script = format!(
                                    "window.__feather_refresh({})",
                                    serde_json::to_string(&content).unwrap()
                                );
                                let _ = webview.evaluate_script(&script);
                            }
                        }
                    }
                    AppEvent::OpenFile(path) => {
                        if let Ok(content) = read_file_with_encoding(&path) {
                            let path_str = path.to_string_lossy().to_string();
                            let script = format!(
                                "window.__feather_render({}, {})",
                                serde_json::to_string(&content).unwrap(),
                                serde_json::to_string(&path_str).unwrap()
                            );
                            let _ = webview.evaluate_script(&script);

                            *current_file.lock().unwrap() = Some(path.clone());
                            *file_dir.lock().unwrap() = path.parent().map(|p| p.to_path_buf());
                            *file_watcher.lock().unwrap() =
                                FileWatcher::new(&path, event_loop_proxy.clone()).ok();
                        }
                    }
                },
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    let size = window.inner_size();
                    if let Ok(pos) = window.outer_position() {
                        let mut config = Config::load();
                        config.update_window_state(WindowState {
                            width: size.width as f64,
                            height: size.height as f64,
                            x: pos.x as f64,
                            y: pos.y as f64,
                        });
                    }
                    *control_flow = ControlFlow::Exit;
                }
                _ => (),
            }
        });
    }

    pub fn register_file_association(&self) -> Result<(), String> {
        let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
        file_association::register_association(&exe_path.to_string_lossy())
    }

    pub fn unregister_file_association(&self) -> Result<(), String> {
        file_association::unregister_association()
    }
}

const INIT_SCRIPT: &str = r#"
window.__feather_ipc = {
    postMessage: function(msg) {
        window.ipc.postMessage(msg);
    }
};
"#;

fn handle_ipc(message: &str, event_loop_proxy: &EventLoopProxy<AppEvent>) {
    let msg: serde_json::Value = match serde_json::from_str(message) {
        Ok(v) => v,
        Err(_) => return,
    };

    let msg_type = msg["type"].as_str().unwrap_or("");

    match msg_type {
        "frontend-ready" => {
            let _ = event_loop_proxy.send_event(AppEvent::FrontendReady);
        }
        "theme-changed" => {
            if let Some(theme) = msg["theme"].as_str() {
                let mut config = Config::load();
                config.update_theme(theme);
            }
        }
        "open-file" => {
            let proxy = event_loop_proxy.clone();
            std::thread::spawn(move || {
                let file = FileDialog::new()
                    .add_filter("Markdown", &["md", "mdx", "markdown"])
                    .set_title("打开 Markdown 文件")
                    .pick_file();

                if let Some(path) = file {
                    let _ = proxy.send_event(AppEvent::OpenFile(path));
                }
            });
        }
        "open-folder" => {
            if let Some(path) = msg["path"].as_str() {
                let _ = open_in_explorer(path);
            }
        }
        "open-editor" => {
            if let Some(path) = msg["path"].as_str() {
                let _ = open_in_editor(path);
            }
        }
        _ => {}
    }
}

fn handle_custom_protocol(
    request: wry::http::Request<Vec<u8>>,
    file_dir: &Arc<Mutex<Option<PathBuf>>>,
) -> wry::http::Response<std::borrow::Cow<'static, [u8]>> {
    let path_str = request.uri().path();
    let path = path_str.strip_prefix('/').unwrap_or(path_str);
    let path = percent_decode_str(path).decode_utf8_lossy().to_string();

    let (data, mime) = if path.is_empty() || path == "index.html" {
        (Assets::index_html().unwrap_or_default().into_bytes(), "text/html")
    } else if let Some(content) = Assets::get_asset(&path) {
        let mime = guess_mime(&path);
        (content, mime)
    } else if path.starts_with("local/") {
        let relative = path.strip_prefix("local/").unwrap_or("");
        if let Some(dir) = file_dir.lock().unwrap().as_ref() {
            let full_path = dir.join(relative);
            if full_path.exists() {
                if let Ok(bytes) = fs::read(&full_path) {
                    let mime = guess_mime(&full_path.to_string_lossy());
                    (bytes, mime)
                } else {
                    (b"Cannot read file".to_vec(), "text/plain")
                }
            } else {
                (b"Not Found".to_vec(), "text/plain")
            }
        } else {
            (b"Not Found".to_vec(), "text/plain")
        }
    } else {
        (b"Not Found".to_vec(), "text/plain")
    };

    HttpResponse::builder()
        .header(CONTENT_TYPE, mime)
        .body(data.into())
        .unwrap()
}

fn guess_mime(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("html") | Some("htm") => "text/html",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}

pub fn read_file_with_encoding(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("Cannot read file: {}", e))?;
    if let Ok(content) = String::from_utf8(bytes.clone()) {
        return Ok(content);
    }
    let (cow, ..) = encoding_rs::GBK.decode(&bytes);
    Ok(cow.into_owned())
}

fn open_in_explorer(path: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(parent) = Path::new(path).parent() {
            std::process::Command::new("xdg-open")
                .arg(parent)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn open_in_editor(path: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
