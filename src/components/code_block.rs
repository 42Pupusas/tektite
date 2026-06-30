//! [`CodeBlock`] — renders a fenced or indented code block in Obsidian style.
//!
//! The code is painted on a `muted` background with rounded corners.
//! An optional language label appears top-right in `muted_foreground`.
//! The code text is selectable (users can highlight and copy substrings).

use egui::{FontId, Margin, Response, Ui, Vec2, Widget};
use marki_parse::{LineRange, MarkdownFile, PoolLine};

use crate::tokens::Tokens;

/// Source of the code text: either a single contiguous slice or a line pool.
enum CodeSource<'src> {
    Slice(&'src str),
    Lines(LineRange),
}

/// An Obsidian-style fenced / indented code block.
#[must_use = "widgets do nothing unless added to a Ui"]
pub struct CodeBlock<'src> {
    md: &'src MarkdownFile<'src>,
    source: CodeSource<'src>,
    language: Option<&'src str>,
}

impl<'src> CodeBlock<'src> {
    /// Build from a single contiguous source slice (most code blocks).
    pub const fn new(
        md: &'src MarkdownFile<'src>,
        language: Option<&'src str>,
        code: &'src str,
    ) -> Self {
        Self {
            md,
            source: CodeSource::Slice(code),
            language,
        }
    }

    /// Build from a dedented line pool (code blocks nested in containers).
    pub const fn from_lines(
        md: &'src MarkdownFile<'src>,
        language: Option<&'src str>,
        lines: LineRange,
    ) -> Self {
        Self {
            md,
            source: CodeSource::Lines(lines),
            language,
        }
    }
}

impl Widget for CodeBlock<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let body_size = ui.text_style_height(&egui::TextStyle::Body);
        let mono_size = body_size * 0.78;

        // Build the display string.
        let text: String = match &self.source {
            CodeSource::Slice(s) => (*s).to_owned(),
            CodeSource::Lines(range) => {
                let lines: &[PoolLine<'_>] = self.md.code_lines(*range);
                let mut buf = String::new();
                for line in lines {
                    for _ in 0..line.pad {
                        buf.push(' ');
                    }
                    buf.push_str(line.text);
                    buf.push('\n');
                }
                buf
            }
        };

        let padding = Vec2::new(12.0, 10.0);
        let font_id = FontId::monospace(mono_size);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let radius = egui::CornerRadius::same(tokens.radius_md());

        egui::Frame::new()
            .fill(tokens.muted)
            .corner_radius(radius)
            .inner_margin(Margin::symmetric(padding.x as i8, padding.y as i8))
            .show(ui, |ui| {
                // Language label: right-aligned row above the code.
                if let Some(lang) = self.language {
                    let lbl_font = FontId::monospace(body_size * 0.68);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(egui::Label::new(
                            egui::RichText::new(lang)
                                .font(lbl_font)
                                .color(tokens.muted_foreground),
                        ));
                    });
                }

                // Selectable code text — users can highlight and copy
                // individual lines or expressions from code blocks.
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(text)
                            .font(font_id)
                            .color(tokens.muted_foreground),
                    )
                    .selectable(true)
                    .wrap(),
                );
            })
            .response
    }
}
