use std::process::Command;
use anyhow::{Context, Result};
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
            Command::new("ydotoold")
                .spawn()
                .context("Failed to start ydotoold. Make sure it's installed: 'sudo pacman -S ydotool'")?;
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        Ok(())
    }


    pub fn inject_backspaces(&self, count: usize) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        let mut key_args = Vec::with_capacity(count * 2);
        for _ in 0..count {
            key_args.push("14:1");
            key_args.push("14:0");
        }

        Command::new("ydotool")
            .env("YDOTOOL_SOCKET", &self.socket_path)
            .arg("key")
            .args(&key_args)
            .status()
            .context("Failed to inject backspace keys")?;

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