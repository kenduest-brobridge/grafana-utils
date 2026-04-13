//! Shell completion generation for the unified CLI.

use clap::CommandFactory;
use clap_complete::{generate, Shell};

use crate::cli::CliArgs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum CompletionShell {
    Bash,
    Zsh,
}

impl From<CompletionShell> for Shell {
    fn from(value: CompletionShell) -> Self {
        match value {
            CompletionShell::Bash => Shell::Bash,
            CompletionShell::Zsh => Shell::Zsh,
        }
    }
}

pub(crate) fn render_completion_script(shell: CompletionShell) -> String {
    let mut command = CliArgs::command();
    let mut output = Vec::new();
    generate(
        Shell::from(shell),
        &mut command,
        "grafana-util",
        &mut output,
    );
    String::from_utf8(output).expect("clap completion output should be valid UTF-8")
}

#[cfg(test)]
mod tests {
    use super::{render_completion_script, CompletionShell};

    fn assert_common_root_commands(script: &str) {
        for command in [
            "dashboard",
            "datasource",
            "alert",
            "access",
            "status",
            "workspace",
            "config",
            "version",
            "completion",
        ] {
            assert!(
                script.contains(command),
                "completion script should include `{command}`\n{script}"
            );
        }
    }

    #[test]
    fn bash_completion_uses_unified_clap_command_tree() {
        let script = render_completion_script(CompletionShell::Bash);

        assert!(script.contains("grafana-util"));
        assert_common_root_commands(&script);
    }

    #[test]
    fn zsh_completion_uses_unified_clap_command_tree() {
        let script = render_completion_script(CompletionShell::Zsh);

        assert!(script.contains("#compdef grafana-util"));
        assert_common_root_commands(&script);
    }
}
