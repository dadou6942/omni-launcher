//! Thème sombre reprenant les couleurs de la version Python.

use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Stroke};
use std::sync::Arc;

pub const BG_MAIN: Color32 = Color32::from_rgb(0x12, 0x12, 0x12);
pub const BG_BTN: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2a);
pub const BG_BTN_HOVER: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x3a);
pub const BG_TUILE_HOVER: Color32 = Color32::from_rgb(0x1e, 0x1e, 0x1e);
pub const BG_MENU: Color32 = Color32::from_rgb(0x1a, 0x1a, 0x1a);
pub const ACCENT: Color32 = Color32::from_rgb(0xde, 0xff, 0x9a);
pub const FG: Color32 = Color32::WHITE;
pub const FG_MUTED: Color32 = Color32::from_rgb(0x88, 0x88, 0x88);
pub const BORDURE: Color32 = Color32::from_rgb(0x4a, 0x4a, 0x4a);
pub const DANGER: Color32 = Color32::from_rgb(0xff, 0x7a, 0x7a);

/// Famille de police grasse (Segoe UI Bold si disponible).
pub fn gras() -> FontFamily {
    FontFamily::Name("gras".into())
}

pub fn appliquer(ctx: &egui::Context) {
    charger_polices(ctx);

    let mut v = egui::Visuals::dark();
    v.panel_fill = BG_MAIN;
    v.window_fill = BG_MENU;
    v.extreme_bg_color = BG_BTN;
    v.faint_bg_color = BG_MENU;
    v.window_stroke = Stroke::new(1.0, BORDURE);
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(8);
    v.override_text_color = Some(FG);
    v.selection.bg_fill = ACCENT.linear_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.hyperlink_color = ACCENT;

    let w = &mut v.widgets;
    w.inactive.weak_bg_fill = BG_BTN;
    w.inactive.bg_fill = BG_BTN;
    w.inactive.bg_stroke = Stroke::NONE;
    w.hovered.weak_bg_fill = BG_BTN_HOVER;
    w.hovered.bg_fill = BG_BTN_HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0, BORDURE);
    w.active.weak_bg_fill = BG_BTN_HOVER;
    w.active.bg_fill = BG_BTN_HOVER;
    w.active.bg_stroke = Stroke::new(1.0, ACCENT);
    for etat in [
        &mut w.inactive,
        &mut w.hovered,
        &mut w.active,
        &mut w.open,
        &mut w.noninteractive,
    ] {
        etat.corner_radius = CornerRadius::same(6);
    }
    w.noninteractive.bg_stroke = Stroke::new(1.0, BORDURE);

    ctx.set_visuals(v);

    ctx.global_style_mut(|s| {
        s.spacing.button_padding = egui::vec2(14.0, 7.0);
        s.spacing.item_spacing = egui::vec2(10.0, 8.0);
        s.spacing.scroll.bar_width = 8.0;
        s.text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
        s.text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(15.0));
        s.text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(12.5));
        s.text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::new(28.0, gras()));
    });
}

/// Utilise Segoe UI (police de Windows, comme la version Tkinter) si elle
/// existe, en gardant les polices d'egui en secours (émojis, symboles).
fn charger_polices(ctx: &egui::Context) {
    let mut polices = FontDefinitions::default();
    let dossier = std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "C:\\Windows".into())
        .join("Fonts");

    let normal = std::fs::read(dossier.join("segoeui.ttf")).ok();
    let gras_data = std::fs::read(dossier.join("segoeuib.ttf")).ok();

    let secours: Vec<String> = polices
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();

    if let Some(data) = normal {
        polices
            .font_data
            .insert("segoe".into(), Arc::new(FontData::from_owned(data)));
        if let Some(f) = polices.families.get_mut(&FontFamily::Proportional) {
            f.insert(0, "segoe".into());
        }
    }

    let mut famille_gras = secours.clone();
    if let Some(data) = gras_data {
        polices
            .font_data
            .insert("segoe_gras".into(), Arc::new(FontData::from_owned(data)));
        famille_gras.insert(0, "segoe_gras".into());
    } else if polices.font_data.contains_key("segoe") {
        famille_gras.insert(0, "segoe".into());
    }
    polices.families.insert(gras(), famille_gras);

    ctx.set_fonts(polices);
}
