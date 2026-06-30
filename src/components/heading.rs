//! [`Heading`] — renders a Markdown ATX heading (H1–H6) in Obsidian style.
//!
//! Each level has a distinct font size, and H1/H2 get a bottom separator line,
//! mirroring Obsidian's default theme.

use egui::{FontId, Response, Ui, Widget, text::LayoutJob};
use marki_parse::{InlineSpan, MarkdownFile};

use crate::{inline::append_inlines, tokens::Tokens};

/// Heading size multipliers relative to the body text size, for H1–H6.
/// Applied to `ui.text_style_height(&TextStyle::Body)` at render time so
/// headings scale with the user's chosen font size instead of being fixed.
const SIZE_SCALE: [f32; 6] = [1.75, 1.40, 1.15, 1.0, 0.95, 0.9];

/// An Obsidian-style rendered heading.
#[must_use = "widgets do nothing unless added to a Ui"]
pub struct Heading<'src> {
    md: &'src MarkdownFile<'src>,
    level: u8,
    content: InlineSpan,
}

impl<'src> Heading<'src> {
    /// Construct a heading widget.
    pub const fn new(md: &'src MarkdownFile<'src>, level: u8, content: InlineSpan) -> Self {
        Self { md, level, content }
    }
}

/// Top spacing multipliers (× body size) added above headings that are not H1.
/// H1 is the document title — it typically appears first so needs no nudge.
const TOP_MARGIN_SCALE: [f32; 6] = [0.0, 0.9, 0.75, 0.6, 0.5, 0.4];

impl Widget for Heading<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let level = self.level.clamp(1, 6) as usize;
        // Same fix as Paragraph: use the configured point size, not the row
        // height, so heading scale multipliers are relative to the same baseline
        // as body text rather than an already-inflated metric.
        let body = egui::TextStyle::Body.resolve(ui.style()).size;
        let size = body * SIZE_SCALE[level - 1];

        // Push headings (H2–H6) away from whatever precedes them.
        let top = body * TOP_MARGIN_SCALE[level - 1];
        if top > 0.0 {
            ui.add_space(top);
        }

        let base = egui::text::TextFormat {
            font_id: FontId::proportional(size),
            color: tokens.heading,
            line_height: Some(size * 1.2),
            ..Default::default()
        };

        let mut job = LayoutJob::default();
        append_inlines(
            ui.ctx(),
            self.md,
            self.content,
            &mut job,
            &tokens,
            &base,
            &mut vec![],
        );

        // `ui.label` handles font layout internally — no need to touch `Fonts`.
        let response = ui.label(job);

        // H1 / H2 get a bottom separator rule, like Obsidian reading view.
        if level <= 2 && ui.is_rect_visible(response.rect) {
            let y = response.rect.max.y + 2.0;
            ui.painter().hline(
                response.rect.min.x..=response.rect.max.x,
                y,
                egui::Stroke::new(1.0, tokens.border),
            );
            ui.add_space(4.0);
        }

        response
    }
}
