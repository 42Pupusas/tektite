//! tektite gallery — live showcase of every Markdown construct tektite renders.
//!
//! Layout mirrors the glazier gallery: fixed left nav, scrollable right pane,
//! top bar with theme toggle.  Palette is strict B&W monochrome (no purple
//! accent) so the typography is the hero.
//!
//! Run with: `cargo run --example gallery`
#![allow(clippy::too_many_lines)]

use eframe::egui;
use egui::{Color32, Frame, Margin, RichText, Stroke};
use tektite::{MarkdownView, Tokens};

// ---------------------------------------------------------------------------
// Monochrome palette — overrides tektite's default tokens
// ---------------------------------------------------------------------------

const fn mono_tokens(dark: bool) -> Tokens {
    if dark {
        Tokens {
            background: Color32::from_gray(0x10),
            foreground: Color32::from_gray(0xe8),
            heading: Color32::WHITE,
            muted: Color32::from_gray(0x1e),
            muted_foreground: Color32::from_gray(0x80),
            accent: Color32::from_gray(0xcc), // light gray link
            accent_foreground: Color32::from_gray(0x10),
            border: Color32::from_gray(0x2a),
            tag: Color32::from_gray(0x22),
            tag_foreground: Color32::from_gray(0xb0),
            radius: 4.0,
        }
    } else {
        Tokens {
            background: Color32::WHITE,
            foreground: Color32::from_gray(0x10),
            heading: Color32::BLACK,
            muted: Color32::from_gray(0xf4),
            muted_foreground: Color32::from_gray(0x60),
            accent: Color32::from_gray(0x33), // dark gray link
            accent_foreground: Color32::WHITE,
            border: Color32::from_gray(0xd8),
            tag: Color32::from_gray(0xe8),
            tag_foreground: Color32::from_gray(0x33),
            radius: 4.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Sample Markdown per section
// ---------------------------------------------------------------------------

const SAMPLE_HEADINGS: &str = r"
# Heading 1
## Heading 2
### Heading 3
#### Heading 4
##### Heading 5
###### Heading 6
";

const SAMPLE_PARAGRAPH: &str = "
Paragraphs are the basic prose unit. tektite wraps them at the available
width and honours **bold**, *italic*, and `inline code` spans.  Emphasis can be
_nested_ inside a **bold _and italic_** run.

A second paragraph follows after a blank line.  Soft breaks within a
paragraph become spaces; two trailing spaces force a  
hard line break (the line above ends with two spaces).
";

const SAMPLE_INLINE: &str = r"
Inline elements inside a paragraph:

- **Bold** via `**double asterisks**`
- *Italic* via `*single asterisks*` or `_underscores_`
- `Inline code` — muted surface, accent colour
- [A hyperlink](https://obsidian.md) — accent-coloured with underline
- An autolink: <https://github.com>
- Raw HTML verbatim: <kbd>Ctrl+K</kbd>
";

const SAMPLE_CODE: &str = r##"
Fenced code block with a language label:

```rust
fn main() {
    let src = "# Hello\n\nSome **bold** text.";
    let md = marki_parse::MarkdownFile::parse(src);
    println!("{md:?}");
}
```

An indented code block (four spaces):

    fn indented() {
        // no language label, same muted surface
    }
"##;

const SAMPLE_BLOCKQUOTE: &str = r"
> A single-line blockquote.

> A multi-line blockquote.
> It continues on the next line and can contain **inline formatting**,
> `inline code`, and [links](https://obsidian.md).

> Nested blockquotes:
> > Inner quote
> > > Deeply nested
";

const SAMPLE_LISTS: &str = r"
Unordered (tight):

- Alpha
- Beta
- Gamma

Ordered:

1. First item
2. Second item with `inline code`
3. Third item

Nested:

- Parent A
  - Child A1
  - Child A2
- Parent B
  1. Ordered child
  2. Another ordered child

Loose (blank lines between items):

- Item one

- Item two

- Item three
";

const SAMPLE_HR: &str = r"
Before the rule.

---

After the first rule.  Three or more dashes, stars, or underscores work.

***

After the second rule.
";

const SAMPLE_TABLES: &str = r"
A basic table (header + two body rows):

| Syntax | Description |
| --- | --- |
| Header | Title |
| Paragraph | Text |

Per-column alignment via colons in the delimiter row:

| Left | Center | Right |
| :--- | :----: | ----: |
| a | b | c |
| longer cell | mid | 42 |

Cells carry **inline formatting**, `code`, and [links](https://obsidian.md):

| Feature | Status |
| :-- | :-- |
| **Bold** text | done |
| `inline code` | done |
| [a link](https://egui.rs) | done |

Short rows are padded, long rows truncated to the header's column count:

| A | B | C |
| - | - | - |
| only one |
| 1 | 2 | 3 | 4 | 5 |
";

/// Build a GFM pipe table with `cols` columns and `rows` body rows, with
/// cycling alignment and a sprinkle of inline formatting / links. Used for the
/// "Large tables" section to show the widget's self-contained 2-D scrolling.
fn gen_table(cols: usize, rows: usize) -> String {
    use std::fmt::Write as _;
    let mut s = String::from("|");
    for c in 0..cols {
        let _ = write!(s, " Column {c:02} |");
    }
    s.push_str("\n|");
    for c in 0..cols {
        s.push_str(match c % 3 {
            0 => " :--- ",
            1 => " :--: ",
            _ => " ---: ",
        });
        s.push('|');
    }
    s.push('\n');
    for r in 0..rows {
        s.push('|');
        for c in 0..cols {
            match (r + c) % 5 {
                0 => write!(s, " **r{r}c{c}** |"),
                1 => write!(s, " `code_{r}_{c}` |"),
                2 => write!(s, " [link {r}.{c}](https://egui.rs) |"),
                3 => write!(s, " a much longer cell value r{r} c{c} |"),
                _ => write!(s, " {r}·{c} |"),
            }
            .unwrap();
        }
        s.push('\n');
    }
    s
}

const SAMPLE_MIXED: &str = r#"
# A full note

This is a realistic Obsidian note mixing all block types.

## Code

```python
def greet(name: str) -> str:
    return f"Hello, {name}!"

print(greet("world"))
```

## Blockquote

> "Any sufficiently advanced technology is indistinguishable from magic."
> — Arthur C. Clarke

## Task list (rendered as tight unordered)

- Set up tektite
- Parse the document with `marki-parse`
- Render with `MarkdownView`

## Links

- [Obsidian](https://obsidian.md)
- [egui](https://github.com/emilk/egui)

## Comparison

| Parser | Zero-copy | Tables |
| :-- | :-: | :-: |
| marki | yes | yes |
| pulldown-cmark | no | yes |

---

*End of note.*
"#;

// ---------------------------------------------------------------------------
// Section registry
// ---------------------------------------------------------------------------

struct Section {
    label: &'static str,
    description: &'static str,
    markdown: String,
}

/// Build the section registry. Most samples are static; the "Large tables"
/// entry is generated so it can stress the widget's height cap + 2-D scroll.
fn sections() -> Vec<Section> {
    let large_tables = format!(
        "A **wide** table (16 columns). Columns squeeze to a floor, then the \
         table scrolls **horizontally** — and because it is taller than the cap \
         it scrolls **vertically** inside its own frame, staying inline with \
         this prose:\n\n{}\nA **tall** table (40 rows) caps its height and scrolls \
         on Y without pushing the rest of the document down:\n\n{}",
        gen_table(16, 6),
        gen_table(4, 40),
    );

    vec![
        Section {
            label: "Headings",
            description: "H1–H6 with size scale and H1/H2 separator rules.",
            markdown: SAMPLE_HEADINGS.to_owned(),
        },
        Section {
            label: "Paragraphs",
            description: "Body prose with soft and hard breaks.",
            markdown: SAMPLE_PARAGRAPH.to_owned(),
        },
        Section {
            label: "Inline elements",
            description: "Bold, italic, inline code, links, autolinks, raw HTML.",
            markdown: SAMPLE_INLINE.to_owned(),
        },
        Section {
            label: "Code blocks",
            description: "Fenced (with language label) and indented code blocks.",
            markdown: SAMPLE_CODE.to_owned(),
        },
        Section {
            label: "Blockquotes",
            description: "Single, multi-line, and nested blockquotes.",
            markdown: SAMPLE_BLOCKQUOTE.to_owned(),
        },
        Section {
            label: "Lists",
            description: "Unordered, ordered, nested, and loose lists.",
            markdown: SAMPLE_LISTS.to_owned(),
        },
        Section {
            label: "Horizontal rule",
            description: "Thematic breaks via ---, ***, or ___.",
            markdown: SAMPLE_HR.to_owned(),
        },
        Section {
            label: "Tables",
            description: "GFM pipe tables: per-column alignment, inline cells, padding.",
            markdown: SAMPLE_TABLES.to_owned(),
        },
        Section {
            label: "Large tables",
            description: "Self-contained scrolling: wide tables scroll X, tall tables cap height + scroll Y.",
            markdown: large_tables,
        },
        Section {
            label: "Mixed / realistic",
            description: "A full note combining all block types.",
            markdown: SAMPLE_MIXED.to_owned(),
        },
    ]
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

struct App {
    dark: bool,
    selected: usize,
    sections: Vec<Section>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            dark: true,
            selected: 0,
            sections: sections(),
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("tektite gallery")
            .with_inner_size([1100.0, 720.0]),
        ..Default::default()
    };

    eframe::run_native(
        "tektite gallery",
        options,
        Box::new(|cc| {
            mono_tokens(true).install(&cc.egui_ctx);
            Ok(Box::new(App::default()))
        }),
    )
}

// ---------------------------------------------------------------------------
// eframe::App impl
// ---------------------------------------------------------------------------

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let tk = Tokens::get(ui);

        // ── Top bar ─────────────────────────────────────────────────────────
        egui::Panel::top("topbar")
            .frame(
                Frame::new()
                    .fill(tk.background)
                    .inner_margin(Margin::symmetric(24, 10))
                    .stroke(Stroke::new(1.0, tk.border)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("tektite").size(17.0).color(tk.heading));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Obsidian-style Markdown renderer for egui")
                            .size(13.0)
                            .color(tk.muted_foreground),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let label = if self.dark { "☀  Light" } else { "☾  Dark" };
                        if ui.button(label).clicked() {
                            self.dark = !self.dark;
                            mono_tokens(self.dark).install(ui.ctx());
                        }
                    });
                });
            });

        // ── Left sidebar ─────────────────────────────────────────────────────
        egui::Panel::left("sidebar")
            .resizable(false)
            .exact_size(210.0)
            .frame(
                Frame::new()
                    .fill(tk.muted)
                    .inner_margin(Margin::symmetric(12, 16))
                    .stroke(Stroke::new(1.0, tk.border)),
            )
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Components")
                        .size(10.0)
                        .color(tk.muted_foreground),
                );
                ui.add_space(8.0);
                for (i, section) in self.sections.iter().enumerate() {
                    let selected = self.selected == i;
                    let color = if selected { tk.heading } else { tk.foreground };
                    let btn = egui::Button::selectable(
                        selected,
                        RichText::new(section.label).size(13.0).color(color),
                    )
                    .frame(false);
                    if ui
                        .add(btn)
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        self.selected = i;
                    }
                }
            });

        // ── Main content ─────────────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(Frame::new().fill(tk.background))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("main_scroll")
                    .show(ui, |ui| {
                        // Clamp content to a readable max-width and center it.
                        // `vertical_centered` keeps widgets horizontally centered;
                        // `set_max_width` prevents the column from going wider
                        // than 740 pt on large screens.
                        ui.vertical_centered(|ui| {
                            ui.set_max_width(740.0);
                            ui.add_space(28.0);

                            let section = &self.sections[self.selected];

                            // Section header.
                            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                                ui.label(RichText::new(section.label).size(20.0).color(tk.heading));
                                ui.add_space(3.0);
                                ui.label(
                                    RichText::new(section.description)
                                        .size(13.0)
                                        .color(tk.muted_foreground),
                                );
                                ui.add_space(16.0);

                                // Rendered output.
                                render_card(ui, tk, |ui| {
                                    let norm =
                                        marki_parse::MarkdownFile::normalize(&section.markdown);
                                    let md = marki_parse::MarkdownFile::parse(&norm);
                                    ui.add(MarkdownView::new(&md));
                                });

                                ui.add_space(16.0);

                                // Source view — skipped for generated samples
                                // whose raw text would be enormous.
                                if section.markdown.len() <= 2_000 {
                                    source_label(ui, tk);
                                    ui.add_space(6.0);
                                    source_card(ui, tk, section.markdown.trim());
                                }

                                ui.add_space(40.0);
                            });
                        });
                    });
            });
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Render `contents` inside a card with the muted surface and border.
fn render_card(ui: &mut egui::Ui, tk: Tokens, contents: impl FnOnce(&mut egui::Ui)) {
    Frame::new()
        .fill(tk.muted)
        .corner_radius(egui::CornerRadius::same(tk.radius_lg()))
        .stroke(Stroke::new(1.0, tk.border))
        .inner_margin(Margin::same(20))
        .show(ui, contents);
}

/// Render `src` in a monospace card with tighter padding.
fn source_card(ui: &mut egui::Ui, tk: Tokens, src: &str) {
    Frame::new()
        .fill(tk.muted)
        .corner_radius(egui::CornerRadius::same(tk.radius_lg()))
        .stroke(Stroke::new(1.0, tk.border))
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.label(
                RichText::new(src)
                    .font(egui::FontId::monospace(11.5))
                    .color(tk.muted_foreground),
            );
        });
}

fn source_label(ui: &mut egui::Ui, tk: Tokens) {
    ui.label(
        RichText::new("Source")
            .size(11.0)
            .color(tk.muted_foreground),
    );
}
