//! External tool management for pipeline stages.

use kat_core::{KatError, Result};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ExternalTool {
    pub name: String,
    pub command: String,
    pub install_hint: String,
    pub timeout: Option<Duration>,
}

impl ExternalTool {
    pub fn new(name: &str, command: &str, install_hint: &str) -> Self {
        Self {
            name: name.to_string(),
            command: command.to_string(),
            install_hint: install_hint.to_string(),
            timeout: None,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn is_available(&self) -> bool {
        Command::new("which")
            .arg(&self.command)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn execute(&self, args: &[&str]) -> Result<Output> {
        if let Some(timeout) = self.timeout {
            self.execute_with_timeout(args, timeout)
        } else {
            self.execute_simple(args)
        }
    }

    fn execute_simple(&self, args: &[&str]) -> Result<Output> {
        let output = Command::new(&self.command)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(KatError::Io)?;

        if !output.status.success() {
            return Err(KatError::ToolFailed {
                tool: self.name.clone(),
                code: output.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(output)
    }

    fn execute_with_timeout(&self, args: &[&str], timeout: Duration) -> Result<Output> {
        use std::io::Read;

        let mut child = Command::new(&self.command)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(KatError::Io)?;

        let start = std::time::Instant::now();

        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let mut stdout = Vec::new();
                    let mut stderr = Vec::new();

                    if let Some(mut out) = child.stdout.take() {
                        let _ = out.read_to_end(&mut stdout);
                    }
                    if let Some(mut err) = child.stderr.take() {
                        let _ = err.read_to_end(&mut stderr);
                    }

                    let output = Output {
                        status,
                        stdout,
                        stderr: stderr.clone(),
                    };

                    if !status.success() {
                        return Err(KatError::ToolFailed {
                            tool: self.name.clone(),
                            code: status.code().unwrap_or(-1),
                            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                        });
                    }

                    return Ok(output);
                }
                Ok(None) => {
                    if start.elapsed() > timeout {
                        let _ = child.kill();
                        return Err(KatError::ToolTimeout {
                            tool: self.name.clone(),
                            seconds: timeout.as_secs(),
                        });
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => return Err(KatError::Io(e)),
            }
        }
    }
}

pub fn bgone() -> ExternalTool {
    ExternalTool::new("bgone", "bgone", "cargo install bgone")
}

pub fn imagemagick() -> ExternalTool {
    ExternalTool::new("ImageMagick", "magick", "brew install imagemagick")
}

pub fn potrace() -> ExternalTool {
    ExternalTool::new("potrace", "potrace", "brew install potrace")
}

pub fn svgo() -> ExternalTool {
    ExternalTool::new("svgo", "svgo", "npm install -g svgo")
}

pub fn swiftdraw() -> ExternalTool {
    ExternalTool::new("SwiftDraw", "swiftdraw", "brew install swiftdraw")
}
