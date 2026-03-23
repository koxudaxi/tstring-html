use unicode_width::UnicodeWidthStr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Doc {
    Text(String),
    Concat(Vec<Doc>),
    Indent(Box<Doc>),
    Line,
    SoftLine,
    HardLine,
    Group(Box<Doc>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderOptions {
    pub line_length: usize,
    pub indent_width: usize,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            line_length: 80,
            indent_width: 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Flat,
    Break,
}

#[derive(Clone, Debug)]
struct Command<'a> {
    indent: usize,
    mode: Mode,
    doc: &'a Doc,
}

impl Doc {
    #[must_use]
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    #[must_use]
    pub fn concat(parts: Vec<Doc>) -> Self {
        Self::Concat(parts)
    }

    #[must_use]
    pub fn indent(self) -> Self {
        Self::Indent(Box::new(self))
    }

    #[must_use]
    pub fn group(self) -> Self {
        Self::Group(Box::new(self))
    }

    #[must_use]
    pub fn line() -> Self {
        Self::Line
    }

    #[must_use]
    pub fn soft_line() -> Self {
        Self::SoftLine
    }

    #[must_use]
    pub fn hard_line() -> Self {
        Self::HardLine
    }
}

#[must_use]
pub fn render(doc: &Doc, options: RenderOptions) -> String {
    let mut out = String::new();
    let mut width = 0usize;
    let mut stack = vec![Command {
        indent: 0,
        mode: Mode::Break,
        doc,
    }];

    while let Some(command) = stack.pop() {
        match command.doc {
            Doc::Text(text) => {
                out.push_str(text);
                width = width_after_text(width, text);
            }
            Doc::Concat(parts) => {
                for part in parts.iter().rev() {
                    stack.push(Command {
                        indent: command.indent,
                        mode: command.mode,
                        doc: part,
                    });
                }
            }
            Doc::Indent(inner) => {
                stack.push(Command {
                    indent: command.indent + options.indent_width,
                    mode: command.mode,
                    doc: inner,
                });
            }
            Doc::Line => match command.mode {
                Mode::Flat => {
                    out.push(' ');
                    width += 1;
                }
                Mode::Break => {
                    push_newline(&mut out, command.indent);
                    width = command.indent;
                }
            },
            Doc::SoftLine => match command.mode {
                Mode::Flat => {}
                Mode::Break => {
                    push_newline(&mut out, command.indent);
                    width = command.indent;
                }
            },
            Doc::HardLine => {
                push_newline(&mut out, command.indent);
                width = command.indent;
            }
            Doc::Group(inner) => {
                let next_mode = match command.mode {
                    Mode::Flat => Mode::Flat,
                    Mode::Break => {
                        let remaining = options.line_length.saturating_sub(width);
                        if fits(
                            remaining,
                            vec![Command {
                                indent: command.indent,
                                mode: Mode::Flat,
                                doc: inner,
                            }],
                        ) {
                            Mode::Flat
                        } else {
                            Mode::Break
                        }
                    }
                };
                stack.push(Command {
                    indent: command.indent,
                    mode: next_mode,
                    doc: inner,
                });
            }
        }
    }

    out
}

#[must_use]
pub fn flat_width(doc: &Doc) -> Option<usize> {
    let mut remaining = usize::MAX;
    if fits(
        remaining,
        vec![Command {
            indent: 0,
            mode: Mode::Flat,
            doc,
        }],
    ) {
        accumulate_flat_width(doc, &mut remaining)?;
        Some(usize::MAX - remaining)
    } else {
        None
    }
}

#[must_use]
pub fn has_forced_break(doc: &Doc) -> bool {
    match doc {
        Doc::Text(text) => text.contains('\n') || text.contains('\r'),
        Doc::Concat(parts) => parts.iter().any(has_forced_break),
        Doc::Indent(inner) | Doc::Group(inner) => has_forced_break(inner),
        Doc::Line | Doc::SoftLine => false,
        Doc::HardLine => true,
    }
}

fn fits(mut remaining: usize, mut stack: Vec<Command<'_>>) -> bool {
    while let Some(command) = stack.pop() {
        match command.doc {
            Doc::Text(text) => {
                let Some(width) = text_flat_width(text) else {
                    return false;
                };
                if width > remaining {
                    return false;
                }
                remaining -= width;
            }
            Doc::Concat(parts) => {
                for part in parts.iter().rev() {
                    stack.push(Command {
                        indent: command.indent,
                        mode: command.mode,
                        doc: part,
                    });
                }
            }
            Doc::Indent(inner) => stack.push(Command {
                indent: command.indent,
                mode: command.mode,
                doc: inner,
            }),
            Doc::Line => {
                if remaining == 0 {
                    return false;
                }
                remaining -= 1;
            }
            Doc::SoftLine => {}
            Doc::HardLine => return false,
            Doc::Group(inner) => stack.push(Command {
                indent: command.indent,
                mode: Mode::Flat,
                doc: inner,
            }),
        }
    }

    true
}

fn accumulate_flat_width(doc: &Doc, remaining: &mut usize) -> Option<()> {
    match doc {
        Doc::Text(text) => {
            *remaining -= text_flat_width(text)?;
        }
        Doc::Concat(parts) => {
            for part in parts {
                accumulate_flat_width(part, remaining)?;
            }
        }
        Doc::Indent(inner) | Doc::Group(inner) => {
            accumulate_flat_width(inner, remaining)?;
        }
        Doc::Line => *remaining -= 1,
        Doc::SoftLine => {}
        Doc::HardLine => return None,
    }
    Some(())
}

fn push_newline(out: &mut String, indent: usize) {
    out.push('\n');
    for _ in 0..indent {
        out.push(' ');
    }
}

fn width_after_text(current_width: usize, text: &str) -> usize {
    match text.rsplit_once('\n') {
        Some((_, tail)) => tail.width(),
        None => current_width + text.width(),
    }
}

fn text_flat_width(text: &str) -> Option<usize> {
    if text.contains('\n') || text.contains('\r') {
        return None;
    }
    Some(text.width())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_break_when_they_do_not_fit() {
        let doc = Doc::concat(vec![
            Doc::text("<div"),
            Doc::concat(vec![Doc::line(), Doc::text("class=\"hello\"")]).indent(),
            Doc::soft_line(),
            Doc::text(">"),
        ])
        .group();

        assert_eq!(
            render(
                &doc,
                RenderOptions {
                    line_length: 10,
                    indent_width: 2,
                }
            ),
            "<div\n  class=\"hello\"\n>"
        );
    }

    #[test]
    fn text_with_newline_forces_break() {
        let doc = Doc::concat(vec![Doc::text("{\n  value\n}"), Doc::text("</div>")]);
        assert!(has_forced_break(&doc));
        assert_eq!(flat_width(&doc), None);
    }
}
