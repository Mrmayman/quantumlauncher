use owo_colors::{OwoColorize, Style};
use ql_core::print::strip_ansi_codes;
use std::{fmt::Write, io::IsTerminal};

#[must_use]
pub fn render_row(
    width: u16,
    items: &[(String, Option<Style>)],
    list_view: bool,
) -> Option<String> {
    let mut out = String::new();
    let max_widths: Vec<usize> = items
        .iter()
        .map(|n| {
            n.0.lines()
                .map(|n| strip_ansi_codes(n).chars().count())
                .max()
                .map(|n| n + 2)
                .unwrap_or(0)
        })
        .collect();

    if !std::io::stdout().is_terminal() {
        render_row_basic(items, &mut out);
        return Some(out);
    }
    if max_widths.iter().copied().max().unwrap_or(0) > width as usize {
        return if list_view {
            render_row_basic(items, &mut out);
            Some(out)
        } else {
            None
        };
    }

    let max_height = items.iter().map(|n| n.0.lines().count()).max().unwrap_or(0);

    if max_widths.iter().sum::<usize>() > width.into() {
        if list_view {
            for line_i in 0..max_height {
                let mut line_buf = String::new();
                let mut current_width = 0;

                for (item_i, (item, style)) in items.iter().enumerate() {
                    if item.is_empty() {
                        continue;
                    }

                    let cell = pad_line(item.lines().nth(line_i), max_widths[item_i]);
                    let cell_width = strip_ansi_codes(&cell).chars().count();

                    // Check if adding this cell would exceed width
                    if current_width > 0 && current_width + 1 + cell_width > width as usize {
                        // Flush line and reset
                        out.push_str(&line_buf);
                        out.push('\n');
                        line_buf.clear();
                        current_width = 0;
                    }

                    // Add a space if not the first element
                    if current_width > 0 {
                        line_buf.push(' ');
                        current_width += 1;
                    }

                    write_line(&mut line_buf, *style, &cell);
                    current_width += cell_width;
                }

                if !line_buf.is_empty() {
                    out.push_str(&line_buf);
                    out.push('\n');
                }
            }
        } else {
            for (item, style) in items {
                write_line(&mut out, *style, item);
                out.push('\n');
            }
        }
    } else {
        for line_i in 0..max_height {
            for (item_i, (item, style)) in items.iter().enumerate() {
                let padded = pad_line(item.lines().nth(line_i), max_widths[item_i]);
                write_line(&mut out, *style, &padded);
                _ = write!(out, "{}", "".style(Style::new()));
            }
            out.push('\n');
        }
    }

    Some(out)
}

/// Non-terminal (e.g. pipe to file) → no colors, tab-delimited
fn render_row_basic(items: &[(String, Option<Style>)], out: &mut String) {
    for line_i in 0..items.iter().map(|n| n.0.lines().count()).max().unwrap_or(0) {
        let mut first = true;
        for (item, _) in items {
            if !first {
                out.push('\t');
            }
            let line = item.lines().nth(line_i).unwrap_or_default();
            out.push_str(&strip_ansi_codes(line));
            first = false;
        }
        out.push('\n');
    }
}

fn write_line(out: &mut String, style: Option<Style>, line: &str) {
    if let Some(style) = style {
        _ = write!(out, "{}", line.style(style));
    } else {
        out.push_str(&line);
    }
}

fn pad_line(line: Option<&str>, width: usize) -> String {
    let line = line.unwrap_or_default();
    let visible_len = strip_ansi_codes(line).chars().count();
    if visible_len < width {
        let pad = width - visible_len;
        format!("{line}{:pad$}", "")
    } else {
        line.to_owned()
    }
}
