use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClipboardError {
    #[error("Clipboard command failed: {message}")]
    CommandFailed { message: String },
    #[error("No clipboard tool available")]
    NoClipboardTool,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Copy text to the system clipboard.
/// Uses `pbcopy` on macOS and `xclip` on Linux.
pub fn copy_to_clipboard(text: &str) -> Result<(), ClipboardError> {
    let (cmd, args) = clipboard_copy_command()?;

    let mut child = Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    if let Some(ref mut stdin) = child.stdin {
        use std::io::Write;
        stdin.write_all(text.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(ClipboardError::CommandFailed {
            message: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(())
}

/// Read text from the system clipboard.
/// Uses `pbpaste` on macOS and `xclip` on Linux.
pub fn read_from_clipboard() -> Result<String, ClipboardError> {
    let (cmd, args) = clipboard_paste_command()?;

    let output = Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()?;

    if !output.status.success() {
        return Err(ClipboardError::CommandFailed {
            message: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn clipboard_copy_command() -> Result<(&'static str, Vec<&'static str>), ClipboardError> {
    if cfg!(target_os = "macos") {
        Ok(("pbcopy", vec![]))
    } else if cfg!(target_os = "linux") {
        Ok(("xclip", vec!["-selection", "clipboard"]))
    } else {
        Err(ClipboardError::NoClipboardTool)
    }
}

fn clipboard_paste_command() -> Result<(&'static str, Vec<&'static str>), ClipboardError> {
    if cfg!(target_os = "macos") {
        Ok(("pbpaste", vec![]))
    } else if cfg!(target_os = "linux") {
        Ok(("xclip", vec!["-selection", "clipboard", "-o"]))
    } else {
        Err(ClipboardError::NoClipboardTool)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_copy_command_returns_valid() {
        let result = clipboard_copy_command();
        if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
            assert!(result.is_ok());
        }
    }

    #[test]
    fn clipboard_paste_command_returns_valid() {
        let result = clipboard_paste_command();
        if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
            assert!(result.is_ok());
        }
    }

    #[test]
    fn clipboard_error_display() {
        let err = ClipboardError::CommandFailed {
            message: "not found".to_string(),
        };
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn clipboard_error_no_tool() {
        let err = ClipboardError::NoClipboardTool;
        assert!(err.to_string().contains("No clipboard tool"));
    }

    #[test]
    fn clipboard_copy_command_platform_specific() {
        let result = clipboard_copy_command();
        if cfg!(target_os = "macos") {
            let (cmd, args) = result.unwrap();
            assert_eq!(cmd, "pbcopy");
            assert!(args.is_empty());
        } else if cfg!(target_os = "linux") {
            let (cmd, args) = result.unwrap();
            assert_eq!(cmd, "xclip");
            assert!(args.contains(&"-selection"));
        }
    }
}
