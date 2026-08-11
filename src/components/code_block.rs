//! [`CodeBlock`] — renders a fenced or indented code block in Obsidian style.
//!
//! The code is painted on a `muted` background with rounded corners.
//! An optional language label appears top-right in `muted_foreground`.
//! The code text is selectable (users can highlight and copy substrings).

use egui::{FontId, Margin, Response, Ui, Widget};
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

        let margin = Margin::symmetric(12, 10);
        let font_id = FontId::monospace(mono_size);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let radius = egui::CornerRadius::same(tokens.radius_md());

        egui::Frame::new()
            .fill(tokens.muted)
            .corner_radius(radius)
            .inner_margin(margin)
            .show(ui, |ui| {
                // Language label: right-aligned row above the code.
                if let Some(lang) = self.language {
                    let lbl_font = FontId::monospace(body_size * 0.68);
                    // Right-alignment needs a bounded rect to align
                    // *within*: an unbounded one makes the row claim the
                    // whole height on offer, which in a tall host leaves
                    // the code stranded under a screenful of blank fill.
                    let row = egui::vec2(
                        ui.available_width(),
                        ui.text_style_height(&egui::TextStyle::Body),
                    );
                    ui.allocate_ui_with_layout(
                        row,
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.add(egui::Label::new(
                                egui::RichText::new(lang)
                                    .font(lbl_font)
                                    .color(tokens.muted_foreground),
                            ));
                        },
                    );
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

#[cfg(test)]
mod tests {
    use egui::Widget as _;
    use marki_parse::MarkdownFile;

    use super::CodeBlock;

    /// Height of a two-line block rendered in a `ui` offering `avail`
    /// vertical points.
    fn height(language: Option<&str>, avail: f32) -> f32 {
        let md = MarkdownFile::parse("");
        let mut measured = 0.0;
        egui::__run_test_ui(|ui| {
            ui.set_max_height(avail);
            measured = CodeBlock::new(&md, language, "one\ntwo\n")
                .ui(ui)
                .rect
                .height();
        });
        measured
    }

    /// A code block is sized by its code, not by the room it is given.
    ///
    /// The viewer hands blocks a tall `Ui` (a scroll area's full height)
    /// while a chat bubble hands them a short one. A block that grows
    /// with the offer renders compactly in a bubble and stretched full
    /// of blank space in the viewer — the same document, two shapes.
    #[test]
    fn height_does_not_follow_the_available_space() {
        let short = height(Some("rust"), 200.0);
        let tall = height(Some("rust"), 4000.0);
        assert!(
            (short - tall).abs() < 1.0,
            "block stretched from {short} to {tall} on a taller ui",
        );
    }

    /// The language label is a one-line row, not a column that grows.
    ///
    /// Right-alignment needs a bounded rect to align within; given an
    /// unbounded one the row claims every point on offer, which is what
    /// stretched the block.
    #[test]
    fn a_labelled_block_is_no_taller_than_an_unlabelled_one() {
        let bare = height(None, 4000.0);
        let labelled = height(Some("rust"), 4000.0);
        assert!(
            labelled - bare < 40.0,
            "language label added {}pt",
            labelled - bare,
        );
    }
}
