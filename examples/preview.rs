//! `tektite` preview — renders a sample Markdown document in an eframe window.
//!
//! Run with: `cargo run --example preview`

use eframe::egui;
use tektite::{MarkdownView, Tokens};

const SAMPLE: &str = r##"
# tektite

Obsidian-style Markdown renderer for **egui**, powered by `marki-parse`.

## Features

- Zero-copy parsing via `marki-parse`
- Obsidian colour tokens wired into `egui::Visuals`
- Specialized widget per block type

## Code example

```rust
let md = marki_parse::MarkdownFile::parse("# Hello");
MarkdownView::new(&md).ui(ui);
```

## Blockquote

> This is a blockquote.
> It can span **multiple lines** with *inline formatting*.

## Ordered list

1. First item
2. Second item with `inline code`
3. Third item

---

Paragraph after a horizontal rule.  Links look like [this](https://obsidian.md).
"##;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("tektite preview")
            .with_inner_size([760.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "tektite",
        options,
        Box::new(|cc| {
            Tokens::dark().install(&cc.egui_ctx);
            Ok(Box::new(App))
        }),
    )
}

struct App;

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.add_space(32.0);
                ui.vertical(|ui| {
                    let normalized = marki_parse::MarkdownFile::normalize(SAMPLE);
                    let md = marki_parse::MarkdownFile::parse(&normalized);
                    ui.add(MarkdownView::new(&md));
                });
                ui.add_space(32.0);
            });
            ui.add_space(16.0);
        });
    }
}
