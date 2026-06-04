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

/// Read text from the system clipboard, if a clipboard tool is available.
pub fn read_from_clipboard() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        read_stdout("pbpaste", &[])
    }
    #[cfg(target_os = "linux")]
    {
        read_stdout("wl-paste", &["--no-newline"])
            .or_else(|| read_stdout("xclip", &["-selection", "clipboard", "-o"]))
            .or_else(|| read_stdout("xsel", &["--clipboard", "--output"]))
    }
    #[cfg(target_os = "windows")]
    {
        read_stdout(
            "powershell",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Get-Clipboard -Raw",
            ],
        )
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

/// Save a PNG/JPEG image from the clipboard to `path`. Returns true when a file was written.
pub fn save_clipboard_image(path: &Path) -> bool {
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
    #[cfg(target_os = "macos")]
    {
        if Command::new("pngpaste")
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
            && path.is_file()
        {
            return true;
        }
        let path_arg = path
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        let script = format!(
            r#"try
                set imageData to (the clipboard as «class PNGf»)
                if imageData is missing value then return "NO"
                set f to open for access POSIX file "{path_arg}" with write permission
                write imageData to f
                close access f
                return "OK"
            on error
                return "NO"
            end try"#
        );
        read_stdout("osascript", &["-e", &script]).as_deref() == Some("OK") && path.is_file()
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("wl-paste")
            .args(["--type", "image/png"])
            .output()
        {
            if output.status.success() && !output.stdout.is_empty() {
                if std::fs::write(path, &output.stdout).is_ok() {
                    return path.is_file();
                }
            }
        }
        if let Ok(output) = Command::new("xclip")
            .args(["-selection", "clipboard", "-t", "image/png", "-o"])
            .output()
        {
            if output.status.success() && !output.stdout.is_empty() {
                if std::fs::write(path, &output.stdout).is_ok() {
                    return path.is_file();
                }
            }
        }
        false
    }
    #[cfg(target_os = "windows")]
    {
        let path_arg = path.to_string_lossy().replace('\'', "''");
        let script = format!(
            r#"$img = Get-Clipboard -Format Image
if ($null -eq $img) {{ exit 1 }}
$img.Save('{path_arg}', [System.Drawing.Imaging.ImageFormat]::Png)
exit 0"#
        );
        Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
            && path.is_file()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = path;
        false
    }
}

/// Copy text to the system clipboard. Returns false if no clipboard tool is available.
pub fn copy_to_clipboard(text: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        spawn_stdin("pbcopy", &[], text)
    }
    #[cfg(target_os = "linux")]
    {
        if spawn_stdin("wl-copy", &[], text) {
            return true;
        }
        if spawn_stdin("xclip", &["-selection", "clipboard"], text) {
            return true;
        }
        spawn_stdin("xsel", &["--clipboard", "--input"], text)
    }
    #[cfg(target_os = "windows")]
    {
        spawn_stdin("clip", &[], text)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = text;
        false
    }
}

/// Show a desktop notification. Returns false if unsupported or no notifier available.
pub fn notify(title: &str, body: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display notification \"{}\" with title \"{}\"",
            escape_applescript(body),
            escape_applescript(title)
        );
        Command::new("osascript")
            .args(["-e", &script])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("notify-send")
            .args([title, body])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(target_os = "windows")]
    {
        let body = escape_xml(body);
        let title = escape_xml(title);
        let script = format!(
            r#"
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null
$xml = New-Object Windows.Data.Xml.Dom.XmlDocument
$xml.LoadXml(@"
<toast><visual><binding template="ToastText02"><text id="1">{title}</text><text id="2">{body}</text></binding></visual></toast>
"@)
$toast = [Windows.UI.Notifications.ToastNotification]::new($xml)
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier("tick").Show($toast)
"#
        );
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &script,
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = (title, body);
        false
    }
}

#[cfg(target_os = "macos")]
fn escape_applescript(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(target_os = "windows")]
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn read_stdout(cmd: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(cmd).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
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
