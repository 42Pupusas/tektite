//! Inline markdown rendering helpers.
//!
//! [`append_inlines`] walks a slice of [`marki_parse::Inline`] elements and
//! appends them to an [`egui::text::LayoutJob`], applying Obsidian-like
//! typography (bold, italic, links, inline code, soft/hard breaks).
//!
//! It also populates a [`Vec<LinkRange>`] with the character spans of every
//! link/autolink so callers can hit-test a cursor position and open URLs.
//!
//! ## Bold rendering
//!
//! egui's default bundle ships **no** bold font face; using
//! `FontFamily::Name("Bold")` panics at runtime when that family is not
//! registered.  We probe the font atlas once per call-site for a usable bold
//! family, in priority order:
//!
//! 1. `"tektite-bold"` — if the app registered tektite's own bold face.
//! 2. `"glazier-bold"` / `"glazier-semibold"` — **companion awareness**: when
//!    tektite is composed with glazier and the app called
//!    `glazier::install_fonts`, Markdown bold automatically renders with real
//!    weight, no extra setup.
//! 3. `"Bold"` — a generic convention some apps register.
//!
//! If none resolve, bold falls back to the proportional face at a slightly
//! increased letter spacing + the heading colour, so it stays visually
//! distinct and never panics.

use egui::{
    Context, FontId,
    text::{LayoutJob, TextFormat},
};
use marki_parse::{Inline, InlineSpan, MarkdownFile};

use crate::tokens::Tokens;

// ---------------------------------------------------------------------------
// Link range — character span + URL for one link in the layout job
// ---------------------------------------------------------------------------

/// A character-index range inside a `LayoutJob`'s text that corresponds to a
/// hyperlink.  Used by widgets to hit-test cursor positions after layout.
pub struct LinkRange {
    /// Inclusive start character index in the concatenated `job.text`.
    pub char_start: usize,
    /// Exclusive end character index.
    pub char_end: usize,
    /// The URL to open when the span is clicked.
    pub url: String,
}

// ---------------------------------------------------------------------------
// Bold family resolution
// ---------------------------------------------------------------------------

/// Bold-family names probed at render time, in priority order. The first one
/// that is registered on the [`Context`] wins; see the module docs.
const BOLD_FAMILIES: [&str; 4] = ["tektite-bold", "glazier-bold", "glazier-semibold", "Bold"];

/// Resolve the first registered bold family on `ctx`, if any.
///
/// Exposed to the crate so block widgets (e.g. table headers) can render an
/// entire run in real bold weight, falling back the same way inline bold does.
pub fn resolve_bold(ctx: &Context) -> Option<egui::FontFamily> {
    ctx.fonts(|f| {
        let families = f.families();
        BOLD_FAMILIES.iter().find_map(|name| {
            let fam = egui::FontFamily::Name((*name).into());
            families.contains(&fam).then_some(fam)
        })
    })
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Append a span of inlines from `md` into `job`, resolving text formatting
/// against the current `tokens` and base `format`.
///
/// Any link/autolink spans are recorded in `links` as [`LinkRange`] entries
/// whose `char_start`/`char_end` index into `job.text`.
///
/// `ctx` is needed to probe which bold font family (if any) is registered.
pub fn append_inlines(
    ctx: &Context,
    md: &MarkdownFile<'_>,
    span: InlineSpan,
    job: &mut LayoutJob,
    tokens: &Tokens,
    base: &TextFormat,
    links: &mut Vec<LinkRange>,
) {
    let bold = resolve_bold(ctx);
    for inline in md.inlines(span) {
        append_inline(md, inline, job, tokens, base, bold.as_ref(), links);
    }
}

// ---------------------------------------------------------------------------
// Recursive inline renderer
// ---------------------------------------------------------------------------

fn append_inline<'src>(
    md: &MarkdownFile<'src>,
    inline: &Inline<'src>,
    job: &mut LayoutJob,
    tokens: &Tokens,
    base: &TextFormat,
    bold: Option<&egui::FontFamily>,
    links: &mut Vec<LinkRange>,
) {
    match inline {
        Inline::Text(t) => {
            job.append(t, 0.0, base.clone());
        }
        Inline::SoftBreak => {
            job.append(" ", 0.0, base.clone());
        }
        Inline::HardBreak => {
            job.append("\n", 0.0, base.clone());
        }
        Inline::Bold(span) => {
            let fmt = bold_format(tokens, base, bold);
            for child in md.inlines(*span) {
                append_inline(md, child, job, tokens, &fmt, bold, links);
            }
        }
        Inline::Italic(span) => {
            let fmt = TextFormat {
                italics: true,
                ..base.clone()
            };
            for child in md.inlines(*span) {
                append_inline(md, child, job, tokens, &fmt, bold, links);
            }
        }
        Inline::Code(s) => {
            let fmt = TextFormat {
                // `muted_foreground` is the semantic "text on a muted surface"
                // token — readable regardless of the host theme palette.
                // Using `accent` here broke shadcn themes where accent ==
                // the hover-surface gray (same hue as `muted`), making
                // inline code invisible.
                color: tokens.muted_foreground,
                background: tokens.muted,
                font_id: FontId::monospace(base.font_id.size),
                ..base.clone()
            };
            job.append(s, 0.0, fmt);
        }
        Inline::Link { text, url, .. } => {
            let fmt = TextFormat {
                color: tokens.accent,
                underline: egui::Stroke::new(1.0, tokens.accent),
                ..base.clone()
            };
            let char_start = job.text.chars().count();
            if text.is_empty() {
                job.append(url, 0.0, fmt);
            } else {
                for child in md.inlines(*text) {
                    append_inline(md, child, job, tokens, &fmt, bold, links);
                }
            }
            let char_end = job.text.chars().count();
            links.push(LinkRange {
                char_start,
                char_end,
                url: (*url).to_owned(),
            });
        }
        Inline::Image { alt, url, .. } => {
            let fmt = TextFormat {
                color: tokens.muted_foreground,
                italics: true,
                ..base.clone()
            };
            // Prefer the alt text; fall back to the URL when alt is empty.
            let mut label = flatten_inlines(md, *alt);
            if label.is_empty() {
                label.push_str(url);
            }
            job.append(&format!("\u{1f5bc} {label}"), 0.0, fmt);
        }
        Inline::Autolink { target, .. } => {
            let fmt = TextFormat {
                color: tokens.accent,
                underline: egui::Stroke::new(1.0, tokens.accent),
                ..base.clone()
            };
            let char_start = job.text.chars().count();
            job.append(target, 0.0, fmt);
            let char_end = job.text.chars().count();
            links.push(LinkRange {
                char_start,
                char_end,
                url: (*target).to_owned(),
            });
        }
        Inline::RawHtml(s) => {
            let fmt = TextFormat {
                color: tokens.muted_foreground,
                font_id: FontId::monospace(base.font_id.size * 0.9),
                ..base.clone()
            };
            job.append(s, 0.0, fmt);
        }
    }
}

/// Build the [`TextFormat`] for a bold run: real bold weight when a bold font
/// family is registered, otherwise the heading colour plus extra letter
/// spacing so it stays visually distinct without a bold face (see module docs).
pub fn bold_format(
    tokens: &Tokens,
    base: &TextFormat,
    bold: Option<&egui::FontFamily>,
) -> TextFormat {
    bold.map_or_else(
        || TextFormat {
            color: tokens.heading,
            extra_letter_spacing: 0.3,
            ..base.clone()
        },
        |family| TextFormat {
            color: tokens.heading,
            font_id: FontId::new(base.font_id.size, family.clone()),
            ..base.clone()
        },
    )
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Build a plain-text string from an inline span (used for image alt text).
#[must_use]
pub fn flatten_inlines(md: &MarkdownFile<'_>, span: InlineSpan) -> String {
    let mut out = String::new();
    for inline in md.inlines(span) {
        flatten_inline(md, inline, &mut out);
    }
    out
}

fn flatten_inline<'src>(md: &MarkdownFile<'src>, inline: &Inline<'src>, out: &mut String) {
    match inline {
        Inline::Text(t) => out.push_str(t),
        Inline::SoftBreak | Inline::HardBreak => out.push(' '),
        Inline::Bold(span) | Inline::Italic(span) => {
            for child in md.inlines(*span) {
                flatten_inline(md, child, out);
            }
        }
        Inline::Code(s) | Inline::RawHtml(s) => out.push_str(s),
        Inline::Link { text, .. } => {
            for child in md.inlines(*text) {
                flatten_inline(md, child, out);
            }
        }
        Inline::Image { alt, .. } => {
            for child in md.inlines(*alt) {
                flatten_inline(md, child, out);
            }
        }
        Inline::Autolink { target, .. } => out.push_str(target),
    }
}
