//! [`UnorderedList`] and [`OrderedList`] — render Markdown lists in Obsidian style.
//!
//! Bullets use the `•` glyph; ordered items render their start number. Both
//! handle nested sub-lists via recursive [`render_sections`] calls.

use egui::{FontId, Response, Ui, Widget};
use marki_parse::{MarkdownFile, OrderedListDelimiter, SectionRange};

use crate::{render_sections, tokens::Tokens};

/// An Obsidian-style unordered (bullet) list.
#[must_use = "widgets do nothing unless added to a Ui"]
pub struct UnorderedList<'src> {
    md: &'src MarkdownFile<'src>,
    items: SectionRange,
    tight: bool,
}

impl<'src> UnorderedList<'src> {
    /// Construct an unordered list widget.
    pub const fn new(md: &'src MarkdownFile<'src>, tight: bool, items: SectionRange) -> Self {
        Self { md, items, tight }
    }
}

impl Widget for UnorderedList<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        render_list(ui, self.md, self.items, self.tight, None)
    }
}

/// An Obsidian-style ordered (numbered) list.
#[must_use = "widgets do nothing unless added to a Ui"]
pub struct OrderedList<'src> {
    md: &'src MarkdownFile<'src>,
    items: SectionRange,
    tight: bool,
    start: u32,
    delimiter: OrderedListDelimiter,
}

impl<'src> OrderedList<'src> {
    /// Construct an ordered list widget.
    pub const fn new(
        md: &'src MarkdownFile<'src>,
        tight: bool,
        start: u32,
        delimiter: OrderedListDelimiter,
        items: SectionRange,
    ) -> Self {
        Self {
            md,
            items,
            tight,
            start,
            delimiter,
        }
    }
}

impl Widget for OrderedList<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        render_list(
            ui,
            self.md,
            self.items,
            self.tight,
            Some((self.start, self.delimiter)),
        )
    }
}

/// Fixed gutter width reserved for the bullet / number marker, in points.
const BULLET_W: f32 = 20.0;
/// Extra space below each item in a loose list, in points.
const ITEM_GAP: f32 = 2.0;

/// Shared rendering: walks items in the section pool, painting a bullet or
/// number glyph then the item's child sections indented below it.
fn render_list(
    ui: &mut Ui,
    md: &MarkdownFile<'_>,
    items: SectionRange,
    tight: bool,
    ordered: Option<(u32, OrderedListDelimiter)>,
) -> Response {
    let tokens = Tokens::get(ui);
    let body_size = ui.text_style_height(&egui::TextStyle::Body);
    let font_id = FontId::proportional(body_size);

    ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
        let mut counter = ordered.map(|(s, _)| s);

        for item in md.child_sections(items) {
            let marki_parse::Section::ListItem { children } = item else {
                continue;
            };

            // Paint bullet / number.
            let bullet_text = counter.as_mut().map_or_else(
                || "•".to_owned(),
                |n| {
                    let delim = ordered.map_or('.', |(_, d)| {
                        if d == OrderedListDelimiter::Dot {
                            '.'
                        } else {
                            ')'
                        }
                    });
                    let s = format!("{n}{delim}");
                    *n += 1;
                    s
                },
            );

            ui.horizontal(|ui| {
                ui.add_space(4.0);
                // Fixed-width gutter for the marker.
                let (bullet_rect, _) =
                    ui.allocate_exact_size(egui::vec2(BULLET_W, body_size), egui::Sense::hover());
                if ui.is_rect_visible(bullet_rect) {
                    ui.painter().text(
                        egui::pos2(bullet_rect.max.x, bullet_rect.min.y),
                        egui::Align2::RIGHT_TOP,
                        &bullet_text,
                        font_id.clone(),
                        tokens.muted_foreground,
                    );
                }

                // Child sections.
                ui.vertical(|ui| {
                    render_sections(ui, md, *children);
                });
            });

            if !tight {
                ui.add_space(ITEM_GAP);
            }
        }
    })
    .response
}
