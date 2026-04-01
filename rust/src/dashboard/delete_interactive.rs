//! Prompt operators for interactive dashboard delete resolution.
//! This module checks for a usable TTY, asks whether deletion should target a UID or
//! a folder path, and requests the extra confirmation needed before running a live
//! delete. It only gathers and validates inputs; the delete request happens elsewhere.

use std::io::{self, IsTerminal, Write};

use crate::common::{message, Result};

use super::DeleteArgs;

pub(crate) fn prepare_interactive_delete_args(args: &DeleteArgs) -> Result<DeleteArgs> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(message("Dashboard delete interactive mode requires a TTY."));
    }
    let mut resolved = args.clone();
    if resolved.uid.as_deref().unwrap_or("").trim().is_empty()
        && resolved.path.as_deref().unwrap_or("").trim().is_empty()
    {
        let mode = prompt_line("Delete by uid or path? [uid/path]: ")?;
        match mode.trim() {
            "uid" => {
                resolved.uid = Some(prompt_line("Dashboard UID: ")?.trim().to_string());
            }
            "path" => {
                resolved.path = Some(prompt_line("Folder path: ")?.trim().to_string());
            }
            _ => {
                return Err(message(
                    "Interactive dashboard delete expected uid or path.",
                ))
            }
        }
    }
    if !resolved.path.as_deref().unwrap_or("").trim().is_empty() && !resolved.delete_folders {
        resolved.delete_folders = prompt_yes_no("Also delete matching folders? [y/N]: ")?;
    }
    Ok(resolved)
}

pub(crate) fn confirm_live_delete() -> Result<bool> {
    prompt_yes_no("Execute live dashboard delete? [y/N]: ")
}

fn prompt_line(prompt: &str) -> Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line)
}

fn prompt_yes_no(prompt: &str) -> Result<bool> {
    let answer = prompt_line(prompt)?;
    let normalized = answer.trim().to_ascii_lowercase();
    Ok(matches!(normalized.as_str(), "y" | "yes"))
}
