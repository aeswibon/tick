use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Open a file path or URL with the system default handler.
pub fn open_path(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(path).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(path).spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn()?;
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = path;
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "open_path is not supported on this platform",
        ));
    }
    Ok(())
}

pub fn open_url(url: &str) -> std::io::Result<()> {
    open_path(Path::new(url))
}

/// Copy text to the system clipboard. Returns false if no clipboard tool is available.
pub fn copy_to_clipboard(text: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        return spawn_stdin("pbcopy", &[], text);
    }
    #[cfg(target_os = "linux")]
    {
        if spawn_stdin("wl-copy", &[], text) {
            return true;
        }
        if spawn_stdin("xclip", &["-selection", "clipboard"], text) {
            return true;
        }
        return spawn_stdin("xsel", &["--clipboard", "--input"], text);
    }
    #[cfg(target_os = "windows")]
    {
        return spawn_stdin("clip", &[], text);
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = text;
        false
    }
}

fn spawn_stdin(cmd: &str, args: &[&str], text: &str) -> bool {
    let mut child = match Command::new(cmd).args(args).stdin(Stdio::piped()).spawn() {
        Ok(c) => c,
        Err(_) => return false,
    };
    if let Some(mut stdin) = child.stdin.take() {
        if stdin.write_all(text.as_bytes()).is_err() {
            return false;
        }
    }
    child.wait().map(|s| s.success()).unwrap_or(false)
}
