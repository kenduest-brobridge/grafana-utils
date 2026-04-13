use crate::cli_help_examples::colorize_grouped_short_help;

#[derive(Clone, Copy)]
pub(crate) struct GroupedHelpRow {
    pub(crate) name: &'static str,
    pub(crate) summary: &'static str,
}

#[derive(Clone, Copy)]
pub(crate) struct GroupedHelpSection {
    pub(crate) heading: &'static str,
    pub(crate) rows: &'static [GroupedHelpRow],
}

pub(crate) struct GroupedHelpSpec {
    pub(crate) usage: &'static str,
    pub(crate) sections: &'static [GroupedHelpSection],
    pub(crate) footer: &'static [&'static str],
}

pub(crate) fn render_grouped_help_spec(spec: &GroupedHelpSpec) -> String {
    let mut output = format!("Usage: {}\n", spec.usage);
    for section in spec.sections {
        output.push('\n');
        output.push_str(section.heading);
        output.push_str(":\n");
        let width = section
            .rows
            .iter()
            .map(|row| row.name.len())
            .max()
            .unwrap_or(0);
        for row in section.rows {
            if row.summary.is_empty() {
                output.push_str(&format!("  {}\n", row.name));
            } else {
                output.push_str(&format!(
                    "  {name:<width$}  {summary}\n",
                    name = row.name,
                    width = width,
                    summary = row.summary
                ));
            }
        }
    }
    if !spec.footer.is_empty() {
        output.push('\n');
        output.push_str(&spec.footer.join("\n"));
        output.push('\n');
    }
    output
}

pub(crate) fn render_short_help_text(spec: &GroupedHelpSpec, colorize: bool) -> String {
    let text = render_grouped_help_spec(spec);
    if colorize {
        colorize_grouped_short_help(&text)
    } else {
        text
    }
}
