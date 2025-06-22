use std::process::Command;
use anyhow::Result;

pub fn execute_command(command: &str, interactive: bool) -> Result<String> {
    let mut command_builder = Command::new("zsh");
    command_builder.arg("-c").arg(command);

    if interactive {
        // For interactive commands (like sudo), we want to connect them to the terminal's I/O
        let status = command_builder.status()?;
        if status.success() {
            Ok(String::new()) // No output to return
        } else {
            Err(anyhow::anyhow!("Command failed with status: {}", status))
        }
    } else {
        // For non-interactive commands, capture the output
        let output = command_builder.output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Command failed: {}", error))
        }
    }
}

pub fn require_sudo(command: &str) -> bool {
    command.contains("sudo")
}

pub fn check_command_exists(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
} 