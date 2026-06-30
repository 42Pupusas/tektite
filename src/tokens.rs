//! [`Tokens`] — Obsidian-inspired design tokens for tektite.
//!
//! # Compatibility with glazier
//!
//! Both crates read from and write to the **same [`egui::Visuals`] fields** for
//! every token that has a natural mapping.  A user who calls
//! `glazier::shadcn_visuals(dark)` gets a [`Visuals`] from which tektite can
//! read sensible values directly, and vice-versa.
//!
//! The mapping mirror's glazier's:
//!
//! | tektite token       | egui `Visuals` field (same as glazier)           |
//! |---------------------|--------------------------------------------------|
//! | `background`        | `panel_fill`, `window_fill`                      |
//! | `foreground`        | `widgets.noninteractive.fg_stroke`               |
//! | `muted`             | `faint_bg_color`, `code_bg_color`                |
//! | `muted_foreground`  | `weak_text_color`                                |
//! | `accent`            | `widgets.hovered.bg_fill`                        |
//! | `accent_foreground` | `widgets.hovered.fg_stroke`                      |
//! | `border`            | `widgets.noninteractive.bg_stroke`, `window_stroke` |
//! | `radius`            | every `widgets.*.corner_radius` (same formula)   |
//!
//! Tektite-specific tokens (`heading`, `tag`, `tag_foreground`) have no natural
//! `Visuals` slot.  [`Tokens::install`] stores them in [`egui::Memory::data`]
//! so [`Tokens::get`] can restore them.  When tektite was *not* installed (e.g.
//! a glazier-only theme was applied) `get` derives sensible fallbacks:
//! `heading → foreground`, `tag → muted`, `tag_foreground → accent`.

use egui::{Color32, CornerRadius, Id, Stroke, Ui, Visuals};

// ---------------------------------------------------------------------------
// Tektite-only extras — stored in egui::Memory::data
// ---------------------------------------------------------------------------

/// Tokens that have no natural [`egui::Visuals`] slot; stored via
/// [`egui::Memory::data`] by [`Tokens::install`] and restored by
/// [`Tokens::get`].
#[derive(Clone, Copy, Debug, PartialEq)]
struct TektiteExtras {
    heading: Color32,
    tag: Color32,
    tag_foreground: Color32,
}

// ---------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------

/// Obsidian-inspired semantic colour tokens for the Markdown renderer.
///
/// Field names mirror Obsidian's CSS variables.  The `*_foreground` fields are
/// the text/icon colour that should sit on the matching surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tokens {
    /// Canvas background (`--background-primary`).
    pub background: Color32,
    /// Default prose text (`--text-normal`).
    pub foreground: Color32,
    /// Heading text — slightly brighter than body in Obsidian (`--text-title`).
    /// Stored in [`egui::Memory::data`] by [`install`](Self::install);
    /// falls back to `foreground` when read from a plain [`Visuals`].
    pub heading: Color32,
    /// Subtle surface — blockquote background, code block fill (`--background-secondary`).
    pub muted: Color32,
    /// Text on muted surfaces — inline code, blockquote prose.
    pub muted_foreground: Color32,
    /// Interactive accent — links, blockquote bar, inline-code tint
    /// (`--interactive-accent`).
    pub accent: Color32,
    /// Text on an accent-coloured surface.
    pub accent_foreground: Color32,
    /// Structural border — horizontal rules, table lines (`--background-modifier-border`).
    pub border: Color32,
    /// Obsidian `#tag` pill background.
    /// Stored in [`egui::Memory::data`] by [`install`](Self::install);
    /// falls back to `muted` otherwise.
    pub tag: Color32,
    /// Obsidian `#tag` pill text.
    /// Stored in [`egui::Memory::data`] by [`install`](Self::install);
    /// falls back to `accent` otherwise.
    pub tag_foreground: Color32,
    /// Base corner radius in points (applied to code blocks, callout cards).
    pub radius: f32,
}

impl Tokens {
    /// Obsidian **dark** theme defaults.
    #[must_use]
    pub const fn dark() -> Self {
        Self {
            background: Color32::from_rgb(0x1e, 0x1e, 0x1e),
            foreground: Color32::from_rgb(0xdc, 0xdd, 0xde),
            heading: Color32::from_rgb(0xf0, 0xf0, 0xf0),
            muted: Color32::from_rgb(0x2a, 0x2a, 0x2a),
            muted_foreground: Color32::from_rgb(0xa8, 0xa9, 0xaa),
            accent: Color32::from_rgb(0x70, 0x5d, 0xcf),
            accent_foreground: Color32::from_rgb(0xff, 0xff, 0xff),
            border: Color32::from_rgb(0x3d, 0x3d, 0x3d),
            tag: Color32::from_rgb(0x2e, 0x27, 0x45),
            tag_foreground: Color32::from_rgb(0x9b, 0x8f, 0xe8),
            radius: 6.0,
        }
    }

    /// Obsidian **light** theme defaults.
    #[must_use]
    pub const fn light() -> Self {
        Self {
            background: Color32::from_rgb(0xff, 0xff, 0xff),
            foreground: Color32::from_rgb(0x1a, 0x1a, 0x1a),
            heading: Color32::from_rgb(0x0d, 0x0d, 0x0d),
            muted: Color32::from_rgb(0xf2, 0xf2, 0xf2),
            muted_foreground: Color32::from_rgb(0x5c, 0x5c, 0x5c),
            accent: Color32::from_rgb(0x70, 0x5d, 0xcf),
            accent_foreground: Color32::from_rgb(0xff, 0xff, 0xff),
            border: Color32::from_rgb(0xd4, 0xd4, 0xd4),
            tag: Color32::from_rgb(0xe8, 0xe4, 0xf9),
            tag_foreground: Color32::from_rgb(0x4a, 0x3a, 0xaa),
            radius: 6.0,
        }
    }

    /// Pick the default tokens for the given mode (`true` = dark).
    #[must_use]
    pub const fn for_mode(dark: bool) -> Self {
        if dark { Self::dark() } else { Self::light() }
    }

    /// Linearly interpolate every colour token between `self` and `other` by
    /// `t` (0 = `self`, 1 = `other`). The non-colour `radius` follows `self`.
    ///
    /// Mirrors [`glazier::Tokens::lerp`], so a host app can animate a shared
    /// light↔dark switch by blending both crates' palettes each frame and
    /// re-[`install`](Self::install)ing the result.
    #[must_use]
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let f = |a: Color32, b: Color32| a.lerp_to_gamma(b, t);
        Self {
            background: f(self.background, other.background),
            foreground: f(self.foreground, other.foreground),
            heading: f(self.heading, other.heading),
            muted: f(self.muted, other.muted),
            muted_foreground: f(self.muted_foreground, other.muted_foreground),
            accent: f(self.accent, other.accent),
            accent_foreground: f(self.accent_foreground, other.accent_foreground),
            border: f(self.border, other.border),
            tag: f(self.tag, other.tag),
            tag_foreground: f(self.tag_foreground, other.tag_foreground),
            radius: self.radius,
        }
    }

    // -----------------------------------------------------------------------
    // Apply / read
    // -----------------------------------------------------------------------

    /// **Preferred install path.** Writes these tokens into the egui
    /// [`egui::Context`]: applies all shared tokens to [`Visuals`] (same slots
    /// as glazier) and persists the tektite-only extras (`heading`, `tag`,
    /// `tag_foreground`) into [`egui::Memory::data`] so [`Tokens::get`] can
    /// restore them.
    ///
    /// Call once at startup instead of `ctx.set_visuals(obsidian_visuals(..))`:
    ///
    /// ```no_run
    /// # let ctx = egui::Context::default();
    /// tektite::Tokens::dark().install(&ctx);
    /// ```
    pub fn install(&self, ctx: &egui::Context) {
        let dark = ctx.global_style().visuals.dark_mode;
        let mut v = if dark {
            Visuals::dark()
        } else {
            Visuals::light()
        };
        self.apply(&mut v);
        ctx.set_visuals(v);
        ctx.data_mut(|d| {
            d.insert_temp(
                Id::NULL,
                TektiteExtras {
                    heading: self.heading,
                    tag: self.tag,
                    tag_foreground: self.tag_foreground,
                },
            );
        });
    }

    /// Write the shared tokens into an [`egui::Visuals`], using the **same
    /// egui slots that glazier uses** so both libraries read each other's
    /// themes correctly.
    ///
    /// This does **not** store tektite-only extras (`heading`, `tag`,
    /// `tag_foreground`).  Use [`install`](Self::install) for the full round-trip.
    pub fn apply(&self, v: &mut Visuals) {
        // Surfaces — identical mapping to glazier.
        v.panel_fill = self.background;
        v.window_fill = self.background;
        // glazier writes `background` to extreme_bg_color (input field bg).
        // We match that so both crates agree on this slot.
        v.extreme_bg_color = self.background;
        v.faint_bg_color = self.muted;
        v.code_bg_color = self.muted;
        v.weak_text_color = Some(self.muted_foreground);
        // glazier sets hyperlink_color = foreground, not accent.
        // We mirror that so a glazier theme read by tektite stays correct.
        v.hyperlink_color = self.foreground;
        v.window_stroke = Stroke::new(1.0, self.border);

        let r = CornerRadius::same(self.radius_md());

        // Non-interactive (labels, cards) — glazier maps `card` here.
        // We use `background`; the card surface in a Markdown document is the
        // canvas itself.
        let ni = &mut v.widgets.noninteractive;
        ni.bg_fill = self.background;
        ni.weak_bg_fill = self.muted;
        ni.bg_stroke = Stroke::new(1.0, self.border);
        ni.fg_stroke = Stroke::new(1.0, self.foreground);
        ni.corner_radius = r;

        // Inactive (resting controls) — glazier maps `secondary` here.
        let ia = &mut v.widgets.inactive;
        ia.bg_fill = self.muted;
        ia.weak_bg_fill = self.muted;
        ia.bg_stroke = Stroke::new(1.0, self.border);
        ia.fg_stroke = Stroke::new(1.0, self.foreground);
        ia.corner_radius = r;

        // Hovered = accent surface — **same slot as glazier**.
        let hv = &mut v.widgets.hovered;
        hv.bg_fill = self.accent;
        hv.weak_bg_fill = self.accent;
        hv.bg_stroke = Stroke::new(1.0, self.border);
        hv.fg_stroke = Stroke::new(1.0, self.accent_foreground);
        hv.corner_radius = r;

        // Active = accent + focus ring — same as glazier.
        let ac = &mut v.widgets.active;
        ac.bg_fill = self.accent;
        ac.weak_bg_fill = self.accent;
        ac.bg_stroke = Stroke::new(2.0, self.accent);
        ac.fg_stroke = Stroke::new(1.0, self.accent_foreground);
        ac.corner_radius = r;

        // Open (popup/dropdown) — keep consistent with hovered.
        v.widgets.open.bg_fill = self.accent;
        v.widgets.open.weak_bg_fill = self.accent;
        v.widgets.open.fg_stroke = Stroke::new(1.0, self.accent_foreground);
        v.widgets.open.corner_radius = r;
    }

    /// Read shared tokens back from a live [`Visuals`], using the **same slots
    /// as glazier's `from_visuals`**.  The tektite-only extras fall back to
    /// derived values (`heading → foreground`, `tag → muted`,
    /// `tag_foreground → accent`).  Call [`get`](Self::get) instead to also
    /// restore extras stored via [`install`](Self::install).
    #[must_use]
    pub fn from_visuals(v: &Visuals) -> Self {
        let foreground = v.widgets.noninteractive.fg_stroke.color;
        // accent lives in widgets.hovered.bg_fill — the same slot glazier uses.
        let accent = v.widgets.hovered.bg_fill;
        Self {
            background: v.panel_fill,
            foreground,
            // No dedicated Visuals slot; fall back to foreground.
            heading: foreground,
            muted: v.faint_bg_color,
            muted_foreground: v.weak_text_color.unwrap_or_else(|| v.weak_text_color()),
            accent,
            accent_foreground: v.widgets.hovered.fg_stroke.color,
            border: v.widgets.noninteractive.bg_stroke.color,
            // No dedicated Visuals slot; fall back to muted / accent.
            tag: v.faint_bg_color,
            tag_foreground: accent,
            radius: f32::from(v.widgets.inactive.corner_radius.nw) / 0.8,
        }
    }

    /// Resolve the active tokens for a [`Ui`].
    ///
    /// Reads shared tokens from the live [`Visuals`] (compatible with glazier),
    /// then overlays tektite-specific extras that were stored by
    /// [`install`](Self::install) — if any.
    #[must_use]
    pub fn get(ui: &Ui) -> Self {
        Self::resolve(ui.ctx(), ui.visuals())
    }

    /// Resolve the active tokens from a [`Context`](egui::Context) — for code
    /// (modals, off-`Ui` painters) that runs outside a [`Ui`].
    ///
    /// Mirrors [`glazier::Tokens::get_ctx`].
    #[must_use]
    pub fn get_ctx(ctx: &egui::Context) -> Self {
        let visuals = ctx.global_style().visuals.clone();
        Self::resolve(ctx, &visuals)
    }

    /// Shared body of [`get`](Self::get) / [`get_ctx`](Self::get_ctx): read the
    /// glazier-compatible tokens from `visuals`, then overlay any tektite extras
    /// stored by [`install`](Self::install).
    fn resolve(ctx: &egui::Context, visuals: &Visuals) -> Self {
        let base = Self::from_visuals(visuals);
        let extras = ctx.data(|d| d.get_temp::<TektiteExtras>(Id::NULL));
        extras.map_or(base, |e| Self {
            heading: e.heading,
            tag: e.tag,
            tag_foreground: e.tag_foreground,
            ..base
        })
    }

    // -----------------------------------------------------------------------
    // Radius scale
    // -----------------------------------------------------------------------

    /// `radius_sm` = radius × 0.6
    #[must_use]
    pub const fn radius_sm(self) -> u8 {
        to_u8(self.radius * 0.6)
    }

    /// `radius_md` = radius × 0.8 — used for code blocks, callouts.
    #[must_use]
    pub const fn radius_md(self) -> u8 {
        to_u8(self.radius * 0.8)
    }

    /// `radius_lg` = radius (base).
    #[must_use]
    pub const fn radius_lg(self) -> u8 {
        to_u8(self.radius)
    }

    /// `radius_xl` = radius × 1.4 — cards, outer containers.
    #[must_use]
    pub const fn radius_xl(self) -> u8 {
        to_u8(self.radius * 1.4)
    }
}

impl Default for Tokens {
    fn default() -> Self {
        Self::dark()
    }
}

// ---------------------------------------------------------------------------
// Convenience constructors
// ---------------------------------------------------------------------------

/// Build an [`egui::Visuals`] preloaded with tektite's Obsidian palette.
///
/// This only writes the Visuals-compatible slots; tektite-specific extras are
/// **not** stored.  Prefer [`Tokens::install`] at startup for the full token set:
///
/// ```no_run
/// # let ctx = egui::Context::default();
/// tektite::Tokens::dark().install(&ctx);       // full fidelity
/// // — or, for Visuals-only compatibility —
/// ctx.set_visuals(tektite::obsidian_visuals(true));
/// ```
#[must_use]
pub fn obsidian_visuals(dark: bool) -> Visuals {
    let mut v = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    Tokens::for_mode(dark).apply(&mut v);
    v
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
const fn to_u8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shared (glazier-compatible) tokens survive an `apply` → `from_visuals`
    /// round-trip through [`Visuals`]. Extras (`heading`/`tag`/`tag_foreground`)
    /// are deliberately excluded — they have no `Visuals` slot.
    #[test]
    fn shared_tokens_round_trip_through_visuals() {
        for dark in [true, false] {
            let tokens = Tokens::for_mode(dark);
            let mut v = if dark {
                Visuals::dark()
            } else {
                Visuals::light()
            };
            tokens.apply(&mut v);
            let back = Tokens::from_visuals(&v);

            assert_eq!(back.background, tokens.background, "background ({dark})");
            assert_eq!(back.foreground, tokens.foreground, "foreground ({dark})");
            assert_eq!(back.muted, tokens.muted, "muted ({dark})");
            assert_eq!(
                back.muted_foreground, tokens.muted_foreground,
                "muted_foreground ({dark})"
            );
            assert_eq!(back.accent, tokens.accent, "accent ({dark})");
            assert_eq!(
                back.accent_foreground, tokens.accent_foreground,
                "accent_foreground ({dark})"
            );
            assert_eq!(back.border, tokens.border, "border ({dark})");
        }
    }

    /// tektite reads `accent` from the **same** `Visuals` slot glazier writes it
    /// to (`widgets.hovered.bg_fill`). This is the core companion contract: a
    /// glazier theme is legible to tektite.
    #[test]
    fn accent_shares_glaziers_visuals_slot() {
        let mut v = Visuals::dark();
        let probe = Color32::from_rgb(0x12, 0x34, 0x56);
        v.widgets.hovered.bg_fill = probe;
        assert_eq!(Tokens::from_visuals(&v).accent, probe);
    }

    /// `lerp` endpoints are the inputs; the midpoint sits strictly between.
    #[test]
    fn lerp_endpoints_and_midpoint() {
        let dark = Tokens::dark();
        let light = Tokens::light();

        assert_eq!(dark.lerp(&light, 0.0).background, dark.background);
        assert_eq!(dark.lerp(&light, 1.0).background, light.background);

        // Midpoint background lies between the two endpoints' luminance.
        let mid = dark.lerp(&light, 0.5).background;
        assert!(mid.r() > dark.background.r() && mid.r() < light.background.r());
        // radius follows `self`.
        assert!((dark.lerp(&light, 0.5).radius - dark.radius).abs() < f32::EPSILON);
    }
}
