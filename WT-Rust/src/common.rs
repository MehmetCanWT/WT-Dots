use eframe::egui;
use std::fs;
use std::io::ErrorKind;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

/// Return a user-private runtime file path (`$XDG_RUNTIME_DIR/<name>`), falling back to `temp_dir()`.
pub fn runtime_path(name: &str) -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(std::env::temp_dir)
        .join(name)
}

pub struct SocketGuard(PathBuf);

impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Bind a single-instance Unix socket; invokes `on_existing` and returns `None` if an instance is active.
pub fn bind_single_instance_socket<F>(
    sock_path: &Path,
    on_existing: F,
) -> Option<(UnixListener, SocketGuard)>
where
    F: FnOnce(UnixStream),
{
    match UnixListener::bind(sock_path) {
        Ok(l) => Some((l, SocketGuard(sock_path.to_path_buf()))),
        Err(e) if e.kind() == ErrorKind::AddrInUse => {
            if let Ok(stream) = UnixStream::connect(sock_path) {
                on_existing(stream);
                return None;
            }
            let _ = fs::remove_file(sock_path);
            UnixListener::bind(sock_path)
                .ok()
                .map(|l| (l, SocketGuard(sock_path.to_path_buf())))
        }
        Err(_) => None,
    }
}

/// Register bundled `0xProto Nerd Font` as a fallback font in `egui`.
pub fn install_nerd_font(ctx: &egui::Context) {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return;
    };
    let candidates = [
        home.join(".local/share/fonts/0xProto/0xProtoNerdFont-Regular.ttf"),
        home.join("WT-Dots/assets/fonts/0xProto/0xProtoNerdFont-Regular.ttf"),
    ];
    for path in candidates {
        if let Ok(bytes) = fs::read(&path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "nerd_proto".to_owned(),
                egui::FontData::from_owned(bytes).into(),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("nerd_proto".to_owned());
            ctx.set_fonts(fonts);
            break;
        }
    }
}
