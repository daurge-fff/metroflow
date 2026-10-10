//! Small platform helpers that don't belong to a specific subsystem.

/// Best-effort: open a URL in the system browser. Silently does nothing on
/// failure (we never want the game to die because a browser is missing).
pub fn open_url(url: &str) {
    #[cfg(target_os = "android")]
    {
        crate::android::open_url(url);
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn();
    }
    #[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "macos",
        target_os = "windows",
        unix
    )))]
    {
        let _ = url;
    }
}
