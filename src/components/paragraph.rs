//! [`Paragraph`] — renders a Markdown paragraph in Obsidian style.
//!
//! Inline elements (bold, italic, links, inline code) are painted via the
//! shared [`append_inlines`] helper.  Links are **clickable**: we lay the
//! galley out manually, hit-test the pointer against link character ranges,
//! and open the URL with [`ui.ctx().open_url`].  Text is selectable so
//! users can highlight and copy substrings.

use egui::{FontId, Response, Sense, Ui, Widget, text::LayoutJob};
use marki_parse::{InlineSpan, MarkdownFile};

use crate::{inline::append_inlines, tokens::Tokens};

/// An Obsidian-style rendered paragraph.
#[must_use = "widgets do nothing unless added to a Ui"]
pub struct Paragraph<'src> {
    md: &'src MarkdownFile<'src>,
    content: InlineSpan,
}

impl<'src> Paragraph<'src> {
    /// Construct a paragraph widget.
    pub const fn new(md: &'src MarkdownFile<'src>, content: InlineSpan) -> Self {
        Self { md, content }
    }
}

impl Widget for Paragraph<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        // Use the configured point size, not `text_style_height` which returns
        // the full row height (em + internal leading, ~1.2×). That made tektite
        // body text render noticeably larger than plain egui labels at the same
        // TextStyle::Body setting.
        let size = egui::TextStyle::Body.resolve(ui.style()).size;

        let base = egui::text::TextFormat {
            font_id: FontId::proportional(size),
            color: tokens.foreground,
            line_height: Some(size * 1.25),
            ..Default::default()
        };

        let mut job = LayoutJob {
            wrap: egui::text::TextWrapping {
                max_width: ui.available_width(),
                ..Default::default()
            },
            ..Default::default()
        };

        let mut links = Vec::new();
        append_inlines(
            ui.ctx(),
            self.md,
            self.content,
            &mut job,
            &tokens,
            &base,
            &mut links,
        );

        if links.is_empty() {
            // Fast path: no links — plain selectable label.
            return ui.add(egui::Label::new(job).selectable(true));
        }

        // Slow path: lay out the galley ourselves so we can hit-test the
        // pointer against link character ranges.  Also make the text
        // selectable for copy/paste.
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::click_and_drag());

        if ui.is_rect_visible(rect) {
            // Pointer-in-rect → find which link (if any) is under it.
            let hovered_url = response.hover_pos().and_then(|pos| {
                // `galley.cursor_from_pos` gives us the cursor closest to the
                // pointer, then we check if that character index is inside any link.
                let local = pos - rect.min;
                let cursor = galley.cursor_from_pos(local);
                let char_idx = cursor.index; // char-level index into the galley text

                links
                    .iter()
                    .find(|l| char_idx >= l.char_start && char_idx < l.char_end)
                    .map(|l| l.url.clone())
            });

            if hovered_url.is_some() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            if response.clicked()
                && let Some(url) = &hovered_url
            {
                ui.ctx().open_url(egui::OpenUrl::new_tab(url));
            }

            // Render the galley, then overlay text-selection so the
            // user can highlight and copy prose that contains links.
            ui.painter()
                .galley(rect.min, galley.clone(), egui::Color32::PLACEHOLDER);

            egui::text_selection::LabelSelectionState::label_text_selection(
                ui,
                &response,
                rect.min,
                galley,
                egui::Color32::PLACEHOLDER,
                egui::Stroke::NONE,
            );
        }

        response
    }
}
