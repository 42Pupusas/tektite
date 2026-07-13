//! [`Table`] — renders a GFM table in Obsidian style.
//!
//! marki parses pipe tables into a [`Section::Table`] holding a [`SectionRange`]
//! of [`Section::TableRow`]s (header first, then body rows), each holding
//! [`Section::TableCell`]s carrying a [`ColumnAlignment`] and an inline span.
//!
//! We lay the table out by hand rather than leaning on [`egui::Grid`] so we can:
//!
//! * honour per-column **alignment** (left / center / right) from the delimiter
//!   row,
//! * paint **header fill** + **grid borders** with the theme [`Tokens`],
//! * render the header row in real **bold** weight (via the shared bold-family
//!   resolver, same as inline bold), and
//! * keep **links clickable** — every cell's link character ranges are
//!   hit-tested against the pointer exactly like [`crate::Paragraph`].

use std::sync::Arc;

use egui::{FontId, Galley, Rect, Response, Sense, Ui, Vec2, Widget, text::LayoutJob};
use marki_parse::{ColumnAlignment, MarkdownFile, Section, SectionRange};

use crate::{
    inline::{LinkRange, append_inlines, bold_format, resolve_bold},
    tokens::Tokens,
};

/// An Obsidian-style rendered GFM table.
#[must_use = "widgets do nothing unless added to a Ui"]
pub struct Table<'src> {
    md: &'src MarkdownFile<'src>,
    rows: SectionRange,
}

impl<'src> Table<'src> {
    /// Construct a table widget from a [`Section::Table`]'s row range.
    pub const fn new(md: &'src MarkdownFile<'src>, rows: SectionRange) -> Self {
        Self { md, rows }
    }
}

/// Compute each column's width and the total non-content width (`pads`: cell
/// padding + grid borders).
///
/// Columns take their natural unwrapped width while the whole row fits the
/// available width. When it would overflow, every column is scaled down
/// proportionally — but **never below [`MIN_COL_W`]**. Once columns hit that
/// floor the table is allowed to exceed the available width, so the caller's
/// scroll area scrolls it on the **X axis** instead of crushing cells into
/// unreadable slivers. (Past natural width it also wraps and scrolls on Y.)
fn column_widths(ui: &Ui, row_jobs: &[Vec<RawCell>], ncols: usize) -> (Vec<f32>, f32) {
    let ncols_f = u16::try_from(ncols).map_or_else(|_| f32::from(u16::MAX), f32::from);
    let mut natural = vec![0.0f32; ncols];
    for cols in row_jobs {
        for (c, raw) in cols.iter().enumerate() {
            let mut probe = raw.job.clone();
            probe.wrap.max_width = f32::INFINITY;
            let w = ui.fonts_mut(|f| f.layout_job(probe)).size().x;
            natural[c] = natural[c].max(w);
        }
    }

    let avail = ui.available_width();
    // Total non-content width: 2*pad per column + one border per column edge.
    let pads = (ncols_f * 2.0).mul_add(CELL_PAD_X, (ncols_f + 1.0) * BORDER_W);
    let content_budget = avail - pads;
    let natural_sum: f32 = natural.iter().sum();
    if natural_sum <= content_budget {
        return (natural, pads);
    }
    // Squeeze proportionally, but clamp each column to MIN_COL_W. If enough
    // columns hit the floor the total can exceed the budget — that's fine, the
    // table overflows and scrolls horizontally rather than becoming unreadable.
    let scale = content_budget / natural_sum;
    let scaled = natural.iter().map(|w| (w * scale).max(MIN_COL_W)).collect();
    (scaled, pads)
}

/// Horizontal padding inside each cell, in points.
const CELL_PAD_X: f32 = 8.0;
/// Vertical padding inside each cell, in points.
///
/// Matches `CELL_PAD_X` (8.0) rather than sitting at half of it — the old
/// 4.0 read as visibly cramped: with a ~1.25x line-height galley plus only
/// 4pt above and below, rows were barely taller than the text itself and
/// glyph descenders (g, y, p) crowded the grid border. 8.0 gives every row
/// the same breathing room on both axes.
const CELL_PAD_Y: f32 = 8.0;
/// Grid line thickness, in points.
const BORDER_W: f32 = 1.0;
/// Minimum column content width when the table is squeezed, in points. Below
/// this, columns stop shrinking and the table overflows / scrolls on X.
const MIN_COL_W: f32 = 72.0;
/// Maximum on-screen height of a table before it scrolls internally on Y, in
/// points. Keeps a table reading inline with the surrounding prose instead of
/// dominating the viewport.
const MAX_TABLE_H: f32 = 1600.0;

/// A cell after pass 1: an unwrapped layout job plus the metadata needed to
/// size columns and, later, lay it out for real.
struct RawCell {
    job: LayoutJob,
    align: ColumnAlignment,
    links: Vec<LinkRange>,
    header: bool,
}

/// One laid-out cell: its galley plus the metadata needed to place and
/// hit-test it.
struct Cell {
    galley: Arc<Galley>,
    align: ColumnAlignment,
    links: Vec<LinkRange>,
}

/// A fully laid-out row: its cells, whether it is the header, and its height.
struct Row {
    cells: Vec<Cell>,
    header: bool,
    height: f32,
}

impl Widget for Table<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::get(ui);
        let size = egui::TextStyle::Body.resolve(ui.style()).size;

        // Base formats: body cells use the prose colour; header cells render in
        // real bold weight (or the inline-bold fallback) and the heading tint.
        let base = egui::text::TextFormat {
            font_id: FontId::proportional(size),
            color: tokens.foreground,
            line_height: Some(size * 1.25),
            ..Default::default()
        };
        let bold_family = resolve_bold(ui.ctx());
        let header_fmt = bold_format(&tokens, &base, bold_family.as_ref());

        // ---- Pass 1: build one LayoutJob per cell (unwrapped) ----------------
        let mut row_jobs: Vec<Vec<RawCell>> = Vec::new();
        let mut ncols = 0usize;
        for row in self.md.child_sections(self.rows) {
            let Section::TableRow { header, cells } = *row else {
                continue;
            };
            let mut cols = Vec::new();
            for cell in self.md.child_sections(cells) {
                let Section::TableCell { align, content } = *cell else {
                    continue;
                };
                let fmt = if header { &header_fmt } else { &base };
                let mut job = LayoutJob::default();
                let mut links = Vec::new();
                append_inlines(
                    ui.ctx(),
                    self.md,
                    content,
                    &mut job,
                    &tokens,
                    fmt,
                    &mut links,
                );
                cols.push(RawCell {
                    job,
                    align,
                    links,
                    header,
                });
            }
            ncols = ncols.max(cols.len());
            row_jobs.push(cols);
        }

        if ncols == 0 || row_jobs.is_empty() {
            // Nothing to draw — allocate nothing and return an empty response.
            return ui.allocate_response(Vec2::ZERO, Sense::hover());
        }

        // ---- Column widths: natural widths, capped to the available space ----
        let (col_w, pads) = column_widths(ui, &row_jobs, ncols);

        // ---- Pass 2: final wrapped layout + row heights ----------------------
        let mut rows: Vec<Row> = Vec::with_capacity(row_jobs.len());
        for cols in row_jobs {
            let header = cols.first().is_some_and(|c| c.header);
            let mut cells = Vec::with_capacity(ncols);
            let mut row_h = 0.0f32;
            for (c, raw) in cols.into_iter().enumerate() {
                let mut job = raw.job;
                job.wrap.max_width = col_w[c];
                let galley = ui.fonts_mut(|f| f.layout_job(job));
                row_h = row_h.max(galley.size().y);
                cells.push(Cell {
                    galley,
                    align: raw.align,
                    links: raw.links,
                });
            }
            rows.push(Row {
                cells,
                header,
                height: 2.0f32.mul_add(CELL_PAD_Y, row_h),
            });
        }

        // ---- Size the full table --------------------------------------------
        // The true table width: if columns bottomed out at MIN_COL_W the table
        // is wider than the available space and the inner scroll area scrolls
        // it horizontally rather than clipping.
        let table_w: f32 = col_w.iter().sum::<f32>() + pads;
        let table_h: f32 = rows.iter().map(|r| r.height).sum::<f32>() + BORDER_W;

        // ---- Render inside a self-contained, height-capped scroll area -------
        // A table is meant to read *inline* with the surrounding prose, so it
        // never grows taller than MAX_TABLE_H: past that it scrolls internally
        // on Y, and if columns overflowed it scrolls on X too — all without
        // pushing the rest of the document off-screen.
        //
        // `max_h` is the exact frame height we want: the true content height,
        // capped at MAX_TABLE_H. We pin it on BOTH ends of the scroll frame —
        // `max_height` caps the top, `min_scrolled_height` sets the floor.
        // Without the floor, egui sizes the frame to whatever vertical space
        // happens to be left in the parent Ui (`available_rect`), and when the
        // table sits deep inside a vertically-scrolling transcript or file
        // viewer that remaining space is near-zero — so egui clamps the frame
        // down to its 64pt `min_scrolled_size` default and the table renders as
        // an unreadable sliver. Pinning the floor to `max_h` keeps the frame at
        // the table's real height regardless of the parent's leftover space.
        let max_h = MAX_TABLE_H.min(table_h);
        let salt = (self.rows.start, self.rows.len);
        egui::ScrollArea::both()
            .id_salt(salt)
            .max_height(max_h)
            .min_scrolled_height(max_h)
            .auto_shrink([true, true])
            .show(ui, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(table_w, table_h), Sense::click());
                if ui.is_rect_visible(rect) {
                    Self::paint(ui, rect, &col_w, &rows, &tokens, &response);
                }
                response
            })
            .inner
    }
}

impl Table<'_> {
    /// Paint header fill, cell galleys (aligned), grid borders, and resolve a
    /// hovered/clicked link.
    fn paint(
        ui: &Ui,
        rect: Rect,
        col_w: &[f32],
        rows: &[Row],
        tokens: &Tokens,
        response: &Response,
    ) {
        let painter = ui.painter();
        let stroke = egui::Stroke::new(BORDER_W, tokens.border);
        let hover_pos = response.hover_pos();
        let mut hovered_url: Option<String> = None;

        // x-offset of each column's content box (after the left border + pad).
        let col_x = |c: usize| -> f32 {
            let mut x = rect.min.x + BORDER_W;
            for w in &col_w[..c] {
                x += CELL_PAD_X * 2.0 + w + BORDER_W;
            }
            x + CELL_PAD_X
        };

        let mut y = rect.min.y + BORDER_W;
        for row in rows {
            let row_rect = Rect::from_min_max(
                egui::pos2(rect.min.x, y - BORDER_W),
                egui::pos2(rect.max.x, y + row.height),
            );
            if row.header {
                painter.rect_filled(row_rect, 0.0, tokens.muted);
            }

            for (c, cell) in row.cells.iter().enumerate() {
                let cw = col_w[c];
                let gw = cell.galley.size().x;
                let x0 = col_x(c);
                // Horizontal alignment within the column content box.
                let x = match cell.align {
                    ColumnAlignment::Right => x0 + (cw - gw).max(0.0),
                    ColumnAlignment::Center => (cw - gw).max(0.0).mul_add(0.5, x0),
                    ColumnAlignment::Left | ColumnAlignment::None => x0,
                };
                let pos = egui::pos2(x, y + CELL_PAD_Y);
                painter.galley(pos, cell.galley.clone(), egui::Color32::PLACEHOLDER);

                // Link hit-testing: is the pointer inside this cell's galley?
                if let Some(p) = hover_pos
                    && hovered_url.is_none()
                {
                    let local = p - pos;
                    let galley_rect = Rect::from_min_size(pos, cell.galley.size());
                    if galley_rect.contains(p) {
                        let idx = cell.galley.cursor_from_pos(local).index;
                        hovered_url = cell
                            .links
                            .iter()
                            .find(|l| idx >= l.char_start && idx < l.char_end)
                            .map(|l| l.url.clone());
                    }
                }
            }
            y += row.height;
        }

        // ---- Grid borders ----------------------------------------------------
        // Outer frame.
        painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
        // Horizontal rules between rows.
        let mut hy = rect.min.y + BORDER_W;
        for row in &rows[..rows.len().saturating_sub(1)] {
            hy += row.height;
            painter.hline(rect.x_range(), hy, stroke);
        }
        // Vertical rules between columns.
        let mut vx = rect.min.x + BORDER_W;
        for w in &col_w[..col_w.len().saturating_sub(1)] {
            vx += CELL_PAD_X * 2.0 + w + BORDER_W;
            painter.vline(vx, rect.y_range(), stroke);
        }

        // ---- Link interaction ------------------------------------------------
        if hovered_url.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked()
            && let Some(url) = &hovered_url
        {
            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
        }
    }
}

#[cfg(test)]
mod tests {
    use egui::Widget as _;
    use marki_parse::{MarkdownFile, Section};

    use super::Table;

    /// Find the first top-level [`Section::Table`] in `md`, panicking otherwise.
    fn table_rows(md: &MarkdownFile<'_>) -> marki_parse::SectionRange {
        match md
            .sections
            .iter()
            .find(|s| matches!(s, Section::Table { .. }))
        {
            Some(Section::Table { rows, .. }) => *rows,
            _ => panic!("expected a table section"),
        }
    }

    /// Render `src` through the table widget in the egui test harness. Returns
    /// without panicking when layout/painting succeed.
    fn render(src: &str) {
        let md = MarkdownFile::parse(src);
        let rows = table_rows(&md);
        egui::__run_test_ui(|ui| {
            Table::new(&md, rows).ui(ui);
        });
    }

    #[test]
    fn renders_basic_table() {
        render("| a | b |\n| --- | --- |\n| 1 | 2 |\n| 3 | 4 |");
    }

    #[test]
    fn renders_all_alignments() {
        render("| l | c | r |\n| :-- | :-: | --: |\n| 1 | 2 | 3 |");
    }

    #[test]
    fn renders_inline_cells_with_links() {
        render("| x | y |\n| - | - |\n| **b** `c` | [k](https://egui.rs) |");
    }

    #[test]
    fn renders_ragged_rows() {
        // Short row padded, long row truncated by the parser; widget must cope.
        render("| a | b | c |\n| - | - | - |\n| one |\n| 1 | 2 | 3 | 4 |");
    }

    #[test]
    fn renders_header_only_table() {
        render("| only header |\n| --- |");
    }

    /// A very wide table is allowed to overflow the available width — columns
    /// squeeze only down to `MIN_COL_W`, then the table grows past the viewport
    /// so the enclosing scroll area can scroll it on X. Here we assert it does
    /// overflow (rather than crushing 24 columns into a 400pt box).
    #[test]
    fn wide_table_overflows_for_horizontal_scroll() {
        use std::fmt::Write as _;

        // 24 columns of long content in a constrained UI.
        let mut src = String::from("|");
        for c in 0..24 {
            let _ = write!(src, " a long header cell {c} |");
        }
        src.push_str("\n|");
        for _ in 0..24 {
            src.push_str(" --- |");
        }
        src.push_str("\n|");
        for c in 0..24 {
            let _ = write!(src, " some long body value {c} |");
        }
        src.push('\n');

        let md = MarkdownFile::parse(&src);
        let rows = table_rows(&md);
        egui::__run_test_ui(|ui| {
            ui.set_max_width(400.0);
            let resp = Table::new(&md, rows).ui(ui);
            // 24 columns can't fit a usable width in 400pt, so the table must
            // overflow — 24 * MIN_COL_W (72) alone is well over 400.
            assert!(
                resp.rect.width() > 400.0,
                "expected overflow for horizontal scroll, got {}",
                resp.rect.width(),
            );
        });
    }

    /// A very tall table must cap its on-screen height (scrolling internally on
    /// Y) so it reads inline instead of dominating the viewport.
    #[test]
    fn tall_table_caps_height() {
        use std::fmt::Write as _;

        let mut src = String::from("| a | b |\n| --- | --- |\n");
        for r in 0..200 {
            let _ = writeln!(src, "| row {r} | value {r} |");
        }

        let md = MarkdownFile::parse(&src);
        let rows = table_rows(&md);
        egui::__run_test_ui(|ui| {
            let before = ui.cursor().top();
            Table::new(&md, rows).ui(ui);
            let consumed = ui.cursor().top() - before;
            // 200 rows would be thousands of points tall; the widget must cap
            // the space it consumes to at most MAX_TABLE_H.
            assert!(
                consumed <= super::MAX_TABLE_H + 24.0,
                "table consumed {consumed}pt, expected <= ~{}",
                super::MAX_TABLE_H,
            );
        });
    }
}
