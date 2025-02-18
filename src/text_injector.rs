use anyhow::{Context, Result};
use std::process::Command;
pub struct TextInjector {
    socket_path: std::path::PathBuf,
}

impl TextInjector {
    pub fn new() -> Result<Self> {
        let socket_path = std::path::PathBuf::from("/tmp/.ydotool_socket");

        if !socket_path.exists() {
            Self::ensure_ydotoold_running()?;
        }

        Ok(Self { socket_path })
    }

    fn ensure_ydotoold_running() -> Result<()> {
        let status = Command::new("pgrep")
            .arg("ydotoold")
            .status()
            .context("Failed to check ydotoold")?;

        if !status.success() {
            Command::new("ydotoold").spawn().context(
                "Failed to start ydotoold. Make sure it's installed: 'sudo pacman -S ydotool'",
            )?;
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        Ok(())
    }

    pub fn inject_backspaces(&self, count: usize) -> Result<()> {
        if count == 0 {
            return Ok(());
        }

        for _ in 0..count {
            Command::new("ydotool")
                .env("YDOTOOL_SOCKET", &self.socket_path)
                .args(["key", "14:1", "14:0"])
                .status()
                .context("Failed to inject backspace key")?;
        }
        Ok(())
    }

    pub fn inject(&self, text: &str) -> Result<()> {
        Command::new("ydotool")
            .env("YDOTOOL_SOCKET", &self.socket_path)
            .args(["type", "--"])
            .arg(text)
            .status()?;
        Ok(())
    }
}
