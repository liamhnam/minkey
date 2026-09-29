// Cross-platform native dialogs (MessageBox on Windows, AppleScript on macOS)

#[cfg(windows)]
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Shows an information message box
pub fn alert(title: &str, text: &str) {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        MessageBoxW(
            std::ptr::null_mut(),
            wide(text).as_ptr(),
            wide(title).as_ptr(),
            MB_ICONINFORMATION | MB_OK,
        );
    }
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display dialog {} with title {} buttons {{\"OK\"}} default button \"OK\" with icon note",
            applescript_str(text),
            applescript_str(title)
        );
        let _ = run_osascript(&script);
    }
}

/// Shows a Yes/No question box, returns true when the user picks Yes
pub fn confirm(title: &str, text: &str) -> bool {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        return MessageBoxW(
            std::ptr::null_mut(),
            wide(text).as_ptr(),
            wide(title).as_ptr(),
            MB_ICONEXCLAMATION | MB_YESNO,
        ) == IDYES;
    }
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "button returned of (display dialog {} with title {} buttons {{\"Không\", \"Có\"}} default button \"Có\" with icon caution)",
            applescript_str(text),
            applescript_str(title)
        );
        return run_osascript(&script).as_deref() == Some("Có");
    }
    #[allow(unreachable_code)]
    false
}

/// Opens a URL with the system default browser
pub fn open_url(url: &str) {
    #[cfg(windows)]
    let _ = std::process::Command::new("cmd").args(["/C", "start", url]).status();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).status();
}

#[cfg(target_os = "macos")]
fn applescript_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Runs an AppleScript snippet and returns its trimmed stdout (None if cancelled / failed)
#[cfg(target_os = "macos")]
pub fn run_osascript(script: &str) -> Option<String> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// macOS file chooser for macro .txt files
#[cfg(target_os = "macos")]
pub fn choose_file(prompt: &str) -> Option<String> {
    let script = format!(
        "POSIX path of (choose file with prompt {} of type {{\"txt\", \"public.plain-text\"}})",
        applescript_str(prompt)
    );
    run_osascript(&script).filter(|p| !p.is_empty())
}

/// macOS save dialog
#[cfg(target_os = "macos")]
pub fn choose_save_file(prompt: &str, default_name: &str) -> Option<String> {
    let script = format!(
        "POSIX path of (choose file name with prompt {} default name {})",
        applescript_str(prompt),
        applescript_str(default_name)
    );
    run_osascript(&script).filter(|p| !p.is_empty())
}
