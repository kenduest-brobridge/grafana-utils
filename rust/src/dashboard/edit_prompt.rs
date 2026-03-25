use std::io::{self, Write};

use crate::common::Result;

pub(crate) fn prompt_yes_no(label: &str) -> Result<bool> {
    let answer = prompt_line(label)?;
    let answer = answer.trim().to_ascii_lowercase();
    Ok(matches!(answer.as_str(), "y" | "yes"))
}

pub(crate) fn prompt_line(label: &str) -> Result<String> {
    print!("{label}");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line)
}
