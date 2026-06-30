//! [`Blockquote`] — renders a Markdown blockquote in Obsidian style.
//!
//! A 3-pt accent-coloured left bar is drawn using a `Frame` with a left
//! inner-margin that reserves space for the bar.  This avoids the
//! post-hoc coordinate arithmetic that caused the bar to drift when the
//! parent layout had its own margin or alignment.

use egui::{Frame, Margin, Response, Ui, Widget};
use marki_parse::{MarkdownFile, SectionRange};

use crate::{render_sections, tokens::Tokens};

/// An Obsidian-style blockquote container.
#[must_use = "widgets do nothing unless added to a Ui"]
pub struct Blockquote<'src> {
    md: &'src MarkdownFile<'src>,
    children: SectionRange,
}

impl<'src> Blockquote<'src> {
    /// Construct a blockquote widget.
    pub const fn new(md: &'src MarkdownFile<'src>, children: SectionRange) -> Self {
        Self { md, children }
    }
}

/// Width of the accent bar, in points.
const BAR_W: f32 = 3.0;
/// Gap between the accent bar and the quote content, in points.
const BAR_GAP: i8 = 8;
/// Left inner-margin reserved for the bar + gap.
const LEFT: i8 = 3 + BAR_GAP; // BAR_W + BAR_GAP

impl Widget for Blockquote<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);

        // The frame shifts content right by BAR_W + BAR_GAP, giving us a
        // clean left margin to paint the accent bar into.
        let inner = Frame::new()
            .inner_margin(Margin {
                left: LEFT,
                right: 0,
                top: 2,
                bottom: 2,
            })
            .fill(tokens.muted)
            .show(ui, |ui| {
                render_sections(ui, self.md, self.children);
            });

        // Paint the bar over the reserved left-margin strip.
        let r = inner.response.rect;
        let bar_rect = egui::Rect::from_min_max(r.min, egui::pos2(r.min.x + BAR_W, r.max.y));
        ui.painter().rect_filled(bar_rect, 0.0, tokens.accent);

        inner.response
    }
}
