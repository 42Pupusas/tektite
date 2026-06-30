//! tektite — Obsidian-style Markdown renderer for egui.
//!
//! Parse your Markdown with [`marki_parse::MarkdownFile`] then hand the result
//! to [`MarkdownView`] for a full document render, or call [`render_sections`]
//! directly to embed sections inside your own UI.
//!
//! ```no_run
//! use tektite::MarkdownView;
//! use egui::Widget as _;
//! # egui::__run_test_ui(|ui| {
//! let md = marki_parse::MarkdownFile::parse("# Hello\n\nSome **bold** text.");
//! MarkdownView::new(&md).ui(ui);
//! # });
//! ```

pub mod components;
mod inline;
pub mod tokens;

pub use components::{
    Blockquote, CodeBlock, Heading, HorizontalRule, OrderedList, Paragraph, Table, UnorderedList,
};
pub use tokens::{Tokens, obsidian_visuals};

use egui::{Response, Ui, Widget};
use marki_parse::{MarkdownFile, Section, SectionRange};

/// Renders a full Markdown document in Obsidian reading-view style.
///
/// Wrap it in an [`egui::ScrollArea`] for long documents.
///
/// ```no_run
/// use tektite::MarkdownView;
/// use egui::Widget as _;
/// # egui::__run_test_ui(|ui| {
/// let md = marki_parse::MarkdownFile::parse("# Hi\n\nHello world.");
/// MarkdownView::new(&md).ui(ui);
/// # });
/// ```
#[must_use = "widgets do nothing unless added to a Ui"]
pub struct MarkdownView<'src> {
    md: &'src MarkdownFile<'src>,
}

impl<'src> MarkdownView<'src> {
    /// Construct a view for a parsed document.
    pub const fn new(md: &'src MarkdownFile<'src>) -> Self {
        Self { md }
    }
}

impl Widget for MarkdownView<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
            render_document(ui, self.md);
        })
        .response
    }
}

/// Render every top-level section of `md`.
///
/// The top-level sections live in [`MarkdownFile::sections`], which is a
/// separate pool from the [`SectionRange`]-addressed children used by
/// containers, so this is the entry point rather than [`render_sections`].
pub fn render_document(ui: &mut Ui, md: &MarkdownFile<'_>) {
    const SECTION_GAP: f32 = 8.0;
    for section in &md.sections {
        render_one(ui, md, section);
        ui.add_space(SECTION_GAP);
    }
}

/// Render the *direct child* sections referenced by a [`SectionRange`].
///
/// This is the recursive workhorse for container interiors: blockquotes and
/// list items pass their child range here, and headings, paragraphs, code
/// blocks, blockquotes and lists each dispatch to their specialized widget.
///
/// For the top-level document use [`render_document`] instead — top-level
/// sections live in a different pool from [`SectionRange`]-addressed children.
pub fn render_sections(ui: &mut Ui, md: &MarkdownFile<'_>, range: SectionRange) {
    const SECTION_GAP: f32 = 8.0;
    for section in md.child_sections(range) {
        render_one(ui, md, section);
        ui.add_space(SECTION_GAP);
    }
}

fn render_one(ui: &mut Ui, md: &MarkdownFile<'_>, section: &Section<'_>) {
    match *section {
        Section::Heading { level, content } => {
            ui.add(Heading::new(md, level, content));
        }
        Section::Paragraph { content } => {
            ui.add(Paragraph::new(md, content));
        }
        Section::CodeBlock { language, code } => {
            ui.add(CodeBlock::new(md, language, code));
        }
        Section::IndentedCode { code } => {
            ui.add(CodeBlock::new(md, None, code));
        }
        Section::CodeLines { language, lines } => {
            ui.add(CodeBlock::from_lines(md, language, lines));
        }
        Section::Blockquote { children } => {
            ui.add(Blockquote::new(md, children));
        }
        Section::UnorderedList { tight, items } => {
            ui.add(UnorderedList::new(md, tight, items));
        }
        Section::OrderedList {
            start,
            delimiter,
            tight,
            items,
        } => {
            ui.add(OrderedList::new(md, tight, start, delimiter, items));
        }
        Section::ListItem { children } => {
            // ListItems are normally consumed inside list widgets; if we ever
            // hit one at the top level (malformed pool), render its children.
            render_sections(ui, md, children);
        }
        Section::HorizontalRule => {
            ui.add(HorizontalRule);
        }
        Section::Table { rows, .. } => {
            ui.add(Table::new(md, rows));
        }
        Section::TableRow { .. } | Section::TableCell { .. } => {
            // Rows and cells are consumed inside the Table widget; reaching one
            // standalone means a malformed pool — render nothing.
        }
        Section::HtmlBlock { html } => {
            let tokens = Tokens::get(ui);
            ui.label(
                egui::RichText::new(html.to_owned())
                    .monospace()
                    .color(tokens.muted_foreground),
            );
        }
        Section::HtmlLines { lines } => {
            let tokens = Tokens::get(ui);
            let text: String = md
                .code_lines(lines)
                .iter()
                .flat_map(|l| {
                    std::iter::repeat_n(' ', l.pad as usize)
                        .chain(l.text.chars())
                        .chain(std::iter::once('\n'))
                })
                .collect();
            ui.label(
                egui::RichText::new(text)
                    .monospace()
                    .color(tokens.muted_foreground),
            );
        }
    }
}
