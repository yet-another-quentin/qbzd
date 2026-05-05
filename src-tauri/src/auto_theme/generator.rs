//! Generate a complete set of CSS custom properties from a ThemePalette.

use std::collections::HashMap;

use super::{GeneratedTheme, PaletteColor, SystemColorScheme, ThemePalette};

/// Generate a full CSS theme from an extracted palette.
pub fn generate_theme(palette: &ThemePalette, source: &str) -> GeneratedTheme {
    let mut vars = HashMap::new();

    // Backgrounds
    vars.insert("--bg-primary".into(), palette.bg_primary.to_hex());
    vars.insert("--bg-secondary".into(), palette.bg_secondary.to_hex());
    vars.insert("--bg-tertiary".into(), palette.bg_tertiary.to_hex());
    vars.insert("--bg-hover".into(), palette.bg_hover.to_hex());

    // Text colors
    let (text_primary, text_secondary, text_muted, text_disabled) = if palette.is_dark {
        (
            PaletteColor::new(255, 255, 255),
            PaletteColor::new(204, 204, 204),
            PaletteColor::new(136, 136, 136),
            PaletteColor::new(85, 85, 85),
        )
    } else {
        (
            PaletteColor::new(15, 15, 15),
            PaletteColor::new(68, 68, 68),
            PaletteColor::new(102, 102, 102),
            PaletteColor::new(153, 153, 153),
        )
    };

    // Verify each text tier against bg_primary at its target contrast ratio.
    // text_disabled is intentionally low-contrast (visual cue for disabled state).
    let text_primary = ensure_text_contrast(text_primary, &palette.bg_primary, palette.is_dark);
    let text_secondary =
        ensure_text_contrast_target(text_secondary, &palette.bg_primary, palette.is_dark, 4.5);
    let text_muted =
        ensure_text_contrast_target(text_muted, &palette.bg_primary, palette.is_dark, 3.0);

    vars.insert("--text-primary".into(), text_primary.to_hex());
    vars.insert("--text-secondary".into(), text_secondary.to_hex());
    vars.insert("--text-muted".into(), text_muted.to_hex());
    vars.insert("--text-disabled".into(), text_disabled.to_hex());

    // Accent triplet — pick btn-primary-text once that works across all three states.
    let accent = palette.accent;
    let accent_hover = accent.shift_lightness(0.10);
    let accent_active = accent.shift_lightness(-0.10);
    vars.insert("--accent-primary".into(), accent.to_hex());
    vars.insert("--accent-hover".into(), accent_hover.to_hex());
    vars.insert("--accent-active".into(), accent_active.to_hex());

    let btn_text = pick_btn_text_for_accent_set(&accent, &accent_hover, &accent_active);
    vars.insert("--btn-primary-text".into(), btn_text.to_hex());

    // Status colors with paired FG tokens (--btn-danger-text, --btn-warning-text).
    let (danger, warning) = if palette.is_dark {
        (
            PaletteColor::new(239, 68, 68),
            PaletteColor::new(251, 191, 36),
        )
    } else {
        (
            PaletteColor::new(220, 38, 38),
            PaletteColor::new(217, 119, 6),
        )
    };
    let hover_alpha = if palette.is_dark { 0.2 } else { 0.15 };

    vars.insert("--danger".into(), danger.to_hex());
    vars.insert("--danger-bg".into(), danger.to_rgba(0.1));
    vars.insert("--danger-border".into(), danger.to_rgba(0.3));
    vars.insert("--danger-hover".into(), danger.to_rgba(hover_alpha));
    vars.insert(
        "--btn-danger-text".into(),
        pick_btn_text_for_bg(&danger).to_hex(),
    );

    vars.insert("--warning".into(), warning.to_hex());
    vars.insert("--warning-bg".into(), warning.to_rgba(0.1));
    vars.insert("--warning-border".into(), warning.to_rgba(0.3));
    vars.insert("--warning-hover".into(), warning.to_rgba(hover_alpha));
    vars.insert(
        "--btn-warning-text".into(),
        pick_btn_text_for_bg(&warning).to_hex(),
    );

    // Borders: subtle shifts from bg_primary
    let border_subtle = if palette.is_dark {
        palette.bg_primary.shift_lightness(0.08)
    } else {
        palette.bg_primary.shift_lightness(-0.08)
    };
    let border_strong = if palette.is_dark {
        palette.bg_primary.shift_lightness(0.14)
    } else {
        palette.bg_primary.shift_lightness(-0.14)
    };
    vars.insert("--border-subtle".into(), border_subtle.to_hex());
    vars.insert("--border-strong".into(), border_strong.to_hex());

    // Alpha tokens: white-based for dark, black-based for light
    let alpha_base = if palette.is_dark {
        (255, 255, 255)
    } else {
        (0, 0, 0)
    };

    let alpha_levels: &[(f64, &str)] = &[
        (0.04, "--alpha-4"),
        (0.05, "--alpha-5"),
        (0.06, "--alpha-6"),
        (0.08, "--alpha-8"),
        (0.10, "--alpha-10"),
        (0.15, "--alpha-15"),
        (0.18, "--alpha-18"),
        (0.20, "--alpha-20"),
        (0.25, "--alpha-25"),
        (0.30, "--alpha-30"),
        (0.35, "--alpha-35"),
        (0.40, "--alpha-40"),
        (0.45, "--alpha-45"),
        (0.50, "--alpha-50"),
        (0.60, "--alpha-60"),
        (0.70, "--alpha-70"),
        (0.80, "--alpha-80"),
        (0.85, "--alpha-85"),
        (0.90, "--alpha-90"),
        (0.95, "--alpha-95"),
    ];

    for (alpha, name) in alpha_levels {
        vars.insert(
            name.to_string(),
            format!(
                "rgba({}, {}, {}, {})",
                alpha_base.0, alpha_base.1, alpha_base.2, alpha
            ),
        );
    }

    GeneratedTheme {
        variables: vars,
        is_dark: palette.is_dark,
        source: source.to_string(),
    }
}

/// Generate a full CSS theme directly from a DE color scheme.
///
/// Instead of k-means extraction, this maps the DE's semantic colors (window bg,
/// view bg, button bg, selection, etc.) directly to CSS variables — producing a
/// theme that matches the system look exactly.
pub fn generate_theme_from_scheme(scheme: &SystemColorScheme) -> GeneratedTheme {
    let mut vars = HashMap::new();

    // Determine dark/light from the window background
    let window_bg = scheme.window_bg.unwrap_or(PaletteColor::new(40, 40, 40));
    let is_dark = window_bg.luminance() < 0.5;

    // ── Backgrounds ──────────────────────────────────────────────────────
    vars.insert("--bg-primary".into(), window_bg.to_hex());

    let bg_secondary = scheme.view_bg.unwrap_or_else(|| {
        if is_dark {
            window_bg.shift_lightness(0.03)
        } else {
            window_bg.shift_lightness(-0.03)
        }
    });
    vars.insert("--bg-secondary".into(), bg_secondary.to_hex());

    let bg_tertiary = scheme.button_bg.unwrap_or_else(|| {
        if is_dark {
            window_bg.shift_lightness(0.10)
        } else {
            window_bg.shift_lightness(-0.10)
        }
    });
    vars.insert("--bg-tertiary".into(), bg_tertiary.to_hex());

    // Hover: midpoint between primary and secondary
    let bg_hover = scheme.window_bg_alt.unwrap_or_else(|| {
        PaletteColor::new(
            ((window_bg.r as u16 + bg_secondary.r as u16) / 2) as u8,
            ((window_bg.g as u16 + bg_secondary.g as u16) / 2) as u8,
            ((window_bg.b as u16 + bg_secondary.b as u16) / 2) as u8,
        )
    });
    vars.insert("--bg-hover".into(), bg_hover.to_hex());

    // ── Text ─────────────────────────────────────────────────────────────
    let text_primary = scheme.window_fg.unwrap_or(if is_dark {
        PaletteColor::new(223, 223, 223)
    } else {
        PaletteColor::new(36, 36, 36)
    });
    let text_primary = ensure_text_contrast(text_primary, &window_bg, is_dark);
    vars.insert("--text-primary".into(), text_primary.to_hex());

    // Secondary/muted/disabled — verify contrast (4.5 / 3.0; disabled stays low).
    let text_secondary_raw = scheme
        .view_fg
        .unwrap_or_else(|| text_primary.shift_lightness(if is_dark { -0.10 } else { 0.10 }));
    let text_secondary =
        ensure_text_contrast_target(text_secondary_raw, &window_bg, is_dark, 4.5);
    vars.insert("--text-secondary".into(), text_secondary.to_hex());

    let text_muted_raw = scheme
        .window_fg_inactive
        .unwrap_or_else(|| text_primary.shift_lightness(if is_dark { -0.25 } else { 0.25 }));
    let text_muted = ensure_text_contrast_target(text_muted_raw, &window_bg, is_dark, 3.0);
    vars.insert("--text-muted".into(), text_muted.to_hex());

    let text_disabled = text_muted.shift_lightness(if is_dark { -0.10 } else { 0.10 });
    vars.insert("--text-disabled".into(), text_disabled.to_hex());

    // ── Accent triplet (selection) ───────────────────────────────────────
    let accent = scheme
        .accent
        .or(scheme.selection_bg)
        .unwrap_or(PaletteColor::new(0, 120, 215));
    let accent_hover = scheme
        .selection_hover
        .unwrap_or_else(|| accent.shift_lightness(0.10));
    let accent_active = accent.shift_lightness(-0.10);
    vars.insert("--accent-primary".into(), accent.to_hex());
    vars.insert("--accent-hover".into(), accent_hover.to_hex());
    vars.insert("--accent-active".into(), accent_active.to_hex());

    // Trust DE-provided selection_fg if present — otherwise compute against the
    // full triplet so hover/active states stay legible too.
    let btn_text = scheme
        .selection_fg
        .unwrap_or_else(|| pick_btn_text_for_accent_set(&accent, &accent_hover, &accent_active));
    vars.insert("--btn-primary-text".into(), btn_text.to_hex());

    // ── Status colors ────────────────────────────────────────────────────
    // Use system negative/neutral if available, else fallback to safe defaults
    let danger = scheme.fg_negative.unwrap_or(if is_dark {
        PaletteColor::new(239, 68, 68)
    } else {
        PaletteColor::new(220, 38, 38)
    });
    vars.insert("--danger".into(), danger.to_hex());
    vars.insert("--danger-bg".into(), danger.to_rgba(0.1));
    vars.insert("--danger-border".into(), danger.to_rgba(0.3));
    vars.insert(
        "--danger-hover".into(),
        danger.to_rgba(if is_dark { 0.2 } else { 0.15 }),
    );
    vars.insert(
        "--btn-danger-text".into(),
        pick_btn_text_for_bg(&danger).to_hex(),
    );

    let warning = scheme.fg_neutral.unwrap_or(if is_dark {
        PaletteColor::new(251, 191, 36)
    } else {
        PaletteColor::new(217, 119, 6)
    });
    vars.insert("--warning".into(), warning.to_hex());
    vars.insert("--warning-bg".into(), warning.to_rgba(0.1));
    vars.insert("--warning-border".into(), warning.to_rgba(0.3));
    vars.insert(
        "--warning-hover".into(),
        warning.to_rgba(if is_dark { 0.2 } else { 0.15 }),
    );
    vars.insert(
        "--btn-warning-text".into(),
        pick_btn_text_for_bg(&warning).to_hex(),
    );

    // ── Borders ──────────────────────────────────────────────────────────
    let border_subtle = if is_dark {
        window_bg.shift_lightness(0.06)
    } else {
        window_bg.shift_lightness(-0.06)
    };
    let border_strong = if is_dark {
        window_bg.shift_lightness(0.12)
    } else {
        window_bg.shift_lightness(-0.12)
    };
    vars.insert("--border-subtle".into(), border_subtle.to_hex());
    vars.insert("--border-strong".into(), border_strong.to_hex());

    // ── Alpha tokens ─────────────────────────────────────────────────────
    let alpha_base = if is_dark { (255, 255, 255) } else { (0, 0, 0) };
    let alpha_levels: &[(f64, &str)] = &[
        (0.04, "--alpha-4"),
        (0.05, "--alpha-5"),
        (0.06, "--alpha-6"),
        (0.08, "--alpha-8"),
        (0.10, "--alpha-10"),
        (0.15, "--alpha-15"),
        (0.18, "--alpha-18"),
        (0.20, "--alpha-20"),
        (0.25, "--alpha-25"),
        (0.30, "--alpha-30"),
        (0.35, "--alpha-35"),
        (0.40, "--alpha-40"),
        (0.45, "--alpha-45"),
        (0.50, "--alpha-50"),
        (0.60, "--alpha-60"),
        (0.70, "--alpha-70"),
        (0.80, "--alpha-80"),
        (0.85, "--alpha-85"),
        (0.90, "--alpha-90"),
        (0.95, "--alpha-95"),
    ];
    for (alpha, name) in alpha_levels {
        vars.insert(
            name.to_string(),
            format!(
                "rgba({}, {}, {}, {})",
                alpha_base.0, alpha_base.1, alpha_base.2, alpha
            ),
        );
    }

    GeneratedTheme {
        variables: vars,
        is_dark,
        source: "system-colors".to_string(),
    }
}

/// Pick the best foreground color for text on the given background (typically `accent`).
///
/// Strategy mirrors what we do in the static themes:
///   1. If pure white reaches WCAG AA Large (3:1) on the bg, prefer it. This matches the
///      visual convention "light text on accent button" used across the app, and accepts
///      AA-Large for button labels (which are typically ≥14px bold or ≥18px regular).
///   2. Otherwise, pick whichever of white/black has the higher contrast ratio.
///
/// This avoids the trap of the simple `luminance < 0.5` threshold, which produces
/// catastrophic 2.0:1 ratios for mid-luminance accents like `#cba6f7` (lavender).
fn pick_btn_text_for_bg(bg: &PaletteColor) -> PaletteColor {
    let white = PaletteColor::new(255, 255, 255);
    let black = PaletteColor::new(0, 0, 0);
    let white_ratio = white.contrast_ratio(bg);
    let black_ratio = black.contrast_ratio(bg);

    if white_ratio >= 3.0 {
        white
    } else if black_ratio > white_ratio {
        black
    } else {
        white
    }
}

/// Ensure text has at least WCAG AA contrast (4.5:1) against the background.
fn ensure_text_contrast(text: PaletteColor, bg: &PaletteColor, is_dark: bool) -> PaletteColor {
    ensure_text_contrast_target(text, bg, is_dark, 4.5)
}

/// Ensure text has at least the given contrast ratio against the background.
///
/// Iteratively shifts lightness toward white (dark themes) or black (light themes)
/// until the target is met, then falls back to pure white/black after 20 steps.
///
/// Common targets:
///   - `4.5` — WCAG AA Normal text (use for body/secondary copy)
///   - `3.0` — WCAG AA Large text or UI components (use for muted helper text on
///             buttons ≥14px bold or ≥18px regular)
fn ensure_text_contrast_target(
    text: PaletteColor,
    bg: &PaletteColor,
    is_dark: bool,
    target: f64,
) -> PaletteColor {
    if text.contrast_ratio(bg) >= target {
        return text;
    }

    let (h, s, l) = text.to_hsl();
    let direction = if is_dark { 0.05 } else { -0.05 };
    let mut new_l = l;

    for _ in 0..20 {
        new_l = (new_l + direction).clamp(0.0, 1.0);
        let candidate = PaletteColor::from_hsl(h, s, new_l);
        if candidate.contrast_ratio(bg) >= target {
            return candidate;
        }
    }

    if is_dark {
        PaletteColor::new(255, 255, 255)
    } else {
        PaletteColor::new(0, 0, 0)
    }
}

/// Pick the best foreground text color for the accent triplet (base, hover, active).
///
/// Like `pick_btn_text_for_bg` but considers the worst case across all three
/// accent variants — so the chosen FG works on `:hover` and `:active` states too.
/// Hover shifts lightness +0.10 (worst case for white text on dark accents);
/// active shifts -0.10 (worst case for black text on light accents).
fn pick_btn_text_for_accent_set(
    accent: &PaletteColor,
    accent_hover: &PaletteColor,
    accent_active: &PaletteColor,
) -> PaletteColor {
    let white = PaletteColor::new(255, 255, 255);
    let black = PaletteColor::new(0, 0, 0);

    // Worst case for white text is the lightest variant (lowest delta to white).
    let white_worst = white
        .contrast_ratio(accent)
        .min(white.contrast_ratio(accent_hover))
        .min(white.contrast_ratio(accent_active));
    // Worst case for black text is the darkest variant.
    let black_worst = black
        .contrast_ratio(accent)
        .min(black.contrast_ratio(accent_hover))
        .min(black.contrast_ratio(accent_active));

    if white_worst >= 3.0 {
        white
    } else if black_worst > white_worst {
        black
    } else {
        white
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_theme_dark() {
        let palette = ThemePalette {
            bg_primary: PaletteColor::new(15, 15, 20),
            bg_secondary: PaletteColor::new(26, 26, 30),
            bg_tertiary: PaletteColor::new(42, 42, 48),
            bg_hover: PaletteColor::new(31, 31, 35),
            accent: PaletteColor::new(66, 133, 244),
            is_dark: true,
            all_colors: vec![],
        };

        let theme = generate_theme(&palette, "test");

        assert!(theme.is_dark);
        assert_eq!(theme.source, "test");
        assert!(theme.variables.contains_key("--bg-primary"));
        assert!(theme.variables.contains_key("--accent-primary"));
        assert!(theme.variables.contains_key("--alpha-50"));
        assert!(theme.variables.contains_key("--danger"));
        assert!(theme.variables.contains_key("--border-subtle"));

        // Dark theme should have white-based alphas
        let alpha50 = theme.variables.get("--alpha-50").unwrap();
        assert!(alpha50.starts_with("rgba(255, 255, 255"));
    }

    #[test]
    fn test_generate_theme_light() {
        let palette = ThemePalette {
            bg_primary: PaletteColor::new(245, 245, 245),
            bg_secondary: PaletteColor::new(235, 235, 235),
            bg_tertiary: PaletteColor::new(220, 220, 220),
            bg_hover: PaletteColor::new(240, 240, 240),
            accent: PaletteColor::new(26, 115, 232),
            is_dark: false,
            all_colors: vec![],
        };

        let theme = generate_theme(&palette, "test-light");

        assert!(!theme.is_dark);

        // Light theme should have black-based alphas
        let alpha50 = theme.variables.get("--alpha-50").unwrap();
        assert!(alpha50.starts_with("rgba(0, 0, 0"));

        // Text should be dark
        let text = theme.variables.get("--text-primary").unwrap();
        assert!(text.starts_with("#0"));
    }

    #[test]
    fn test_all_required_tokens_present() {
        let palette = ThemePalette {
            bg_primary: PaletteColor::new(20, 20, 25),
            bg_secondary: PaletteColor::new(30, 30, 35),
            bg_tertiary: PaletteColor::new(45, 45, 50),
            bg_hover: PaletteColor::new(35, 35, 40),
            accent: PaletteColor::new(100, 200, 100),
            is_dark: true,
            all_colors: vec![],
        };

        let theme = generate_theme(&palette, "completeness-test");

        let required = [
            "--bg-primary",
            "--bg-secondary",
            "--bg-tertiary",
            "--bg-hover",
            "--text-primary",
            "--text-secondary",
            "--text-muted",
            "--text-disabled",
            "--accent-primary",
            "--accent-hover",
            "--accent-active",
            "--btn-primary-text",
            "--danger",
            "--danger-bg",
            "--danger-border",
            "--danger-hover",
            "--warning",
            "--warning-bg",
            "--warning-border",
            "--warning-hover",
            "--border-subtle",
            "--border-strong",
            "--alpha-4",
            "--alpha-5",
            "--alpha-6",
            "--alpha-8",
            "--alpha-10",
            "--alpha-15",
            "--alpha-18",
            "--alpha-20",
            "--alpha-25",
            "--alpha-30",
            "--alpha-35",
            "--alpha-40",
            "--alpha-45",
            "--alpha-50",
            "--alpha-60",
            "--alpha-70",
            "--alpha-80",
            "--alpha-85",
            "--alpha-90",
            "--alpha-95",
            "--btn-danger-text",
            "--btn-warning-text",
        ];

        for key in &required {
            assert!(
                theme.variables.contains_key(*key),
                "Missing required token: {}",
                key
            );
        }
    }

    #[test]
    fn pick_btn_text_prefers_white_when_aa_large_passes() {
        // Stratego red: white = 4.14:1 (AA-Large), black = 5.07:1 (AA Normal).
        // The static-theme decision was to keep white for visual consistency.
        let red = PaletteColor::new(0xed, 0x2f, 0x3d);
        assert_eq!(pick_btn_text_for_bg(&red), PaletteColor::new(255, 255, 255));
    }

    #[test]
    fn pick_btn_text_picks_black_when_white_fails_aa_large() {
        // Catppuccin Mocha lavender: white = 2.03:1 (FAIL), black = 8.07:1 (AAA).
        // The old luminance-threshold logic incorrectly picked white here because
        // lum=0.55 ≥ 0.5 — but contrast was catastrophic.
        let lavender = PaletteColor::new(0xcb, 0xa6, 0xf7);
        assert_eq!(pick_btn_text_for_bg(&lavender), PaletteColor::new(0, 0, 0));
    }

    #[test]
    fn pick_btn_text_picks_black_for_dracula_purple() {
        // bd93f9: white = 2.41 (FAIL), black = 8.71 (AAA). Old code picked white
        // (lum 0.43 < 0.5) — wrong.
        let purple = PaletteColor::new(0xbd, 0x93, 0xf9);
        assert_eq!(pick_btn_text_for_bg(&purple), PaletteColor::new(0, 0, 0));
    }

    #[test]
    fn pick_btn_text_handles_high_contrast_yellow() {
        // ffff00: white = 1.07 (utterly fails), black = 19.56 (AAA). Must pick black.
        let yellow = PaletteColor::new(255, 255, 0);
        assert_eq!(pick_btn_text_for_bg(&yellow), PaletteColor::new(0, 0, 0));
    }

    #[test]
    fn pick_btn_text_white_on_dark_navy() {
        // Deep blue: white = ~14:1, black = ~1.5:1. Trivially white.
        let navy = PaletteColor::new(0x12, 0x1a, 0x40);
        assert_eq!(pick_btn_text_for_bg(&navy), PaletteColor::new(255, 255, 255));
    }

    #[test]
    fn ensure_text_contrast_target_aa_normal_meets_45() {
        // Mid-grey text on dark bg: starts at ~3:1, must shift to ≥4.5.
        let bg = PaletteColor::new(20, 20, 20);
        let dim_text = PaletteColor::new(110, 110, 110);
        let result = ensure_text_contrast_target(dim_text, &bg, true, 4.5);
        assert!(
            result.contrast_ratio(&bg) >= 4.5,
            "got {:.2}",
            result.contrast_ratio(&bg)
        );
    }

    #[test]
    fn ensure_text_contrast_target_aa_large_meets_3() {
        // Same dim text but only need AA-Large — should converge faster (or be unchanged
        // if already ≥3).
        let bg = PaletteColor::new(20, 20, 20);
        let dim_text = PaletteColor::new(110, 110, 110);
        let result = ensure_text_contrast_target(dim_text, &bg, true, 3.0);
        assert!(
            result.contrast_ratio(&bg) >= 3.0,
            "got {:.2}",
            result.contrast_ratio(&bg)
        );
    }

    #[test]
    fn pick_btn_text_for_accent_set_holds_across_hover_active() {
        // Catppuccin Mocha lavender + ±10% lightness variants.
        // Black should be picked because white fails everywhere.
        let accent = PaletteColor::new(0xcb, 0xa6, 0xf7);
        let hover = accent.shift_lightness(0.10);
        let active = accent.shift_lightness(-0.10);
        assert_eq!(
            pick_btn_text_for_accent_set(&accent, &hover, &active),
            PaletteColor::new(0, 0, 0)
        );
    }

    #[test]
    fn pick_btn_text_for_accent_set_picks_white_for_deep_navy() {
        // Deep navy stays dark across the triplet — white is the safe choice.
        let accent = PaletteColor::new(0x12, 0x1a, 0x40);
        let hover = accent.shift_lightness(0.10);
        let active = accent.shift_lightness(-0.10);
        assert_eq!(
            pick_btn_text_for_accent_set(&accent, &hover, &active),
            PaletteColor::new(255, 255, 255)
        );
    }

    #[test]
    fn generated_theme_includes_btn_status_text_tokens() {
        let palette = ThemePalette {
            bg_primary: PaletteColor::new(20, 20, 20),
            bg_secondary: PaletteColor::new(30, 30, 30),
            bg_tertiary: PaletteColor::new(45, 45, 45),
            bg_hover: PaletteColor::new(35, 35, 35),
            accent: PaletteColor::new(100, 200, 100),
            is_dark: true,
            all_colors: vec![],
        };
        let theme = generate_theme(&palette, "test");
        assert!(theme.variables.contains_key("--btn-danger-text"));
        assert!(theme.variables.contains_key("--btn-warning-text"));
        // For #ef4444 (dark theme red): white passes AA-Large → expect white.
        assert_eq!(theme.variables.get("--btn-danger-text").unwrap(), "#ffffff");
        // For #fbbf24 (dark theme gold): white fails (1.92), black wins.
        assert_eq!(theme.variables.get("--btn-warning-text").unwrap(), "#000000");
    }
}
