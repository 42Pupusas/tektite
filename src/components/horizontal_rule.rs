//! [`HorizontalRule`] — renders a Markdown `---` thematic break.
//!
//! A 1-pt hairline in `border` colour, matching Obsidian's separator style.

use egui::{Response, Sense, Ui, Vec2, Widget};

use crate::tokens::Tokens;

/// An Obsidian-style horizontal rule.
#[must_use = "widgets do nothing unless added to a Ui"]
pub struct HorizontalRule;

impl Widget for HorizontalRule {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let desired = Vec2::new(ui.available_width(), 12.0);
        let (rect, response) = ui.allocate_at_least(desired, Sense::hover());

        if ui.is_rect_visible(rect) {
            let y = rect.center().y;
            ui.painter().hline(
                rect.min.x..=rect.max.x,
                y,
                egui::Stroke::new(1.0, tokens.border),
            );
        }

        response
    }
}
