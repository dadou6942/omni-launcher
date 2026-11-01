//! Thèmes de couleurs. La palette active est globale : l'interface la relit à
//! chaque image via [`p`], ce qui permet de changer de thème à la volée.

use crate::config::Theme;
use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Stroke};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub bg_main: Color32,
    pub bg_btn: Color32,
    pub bg_btn_hover: Color32,
    pub bg_tuile_hover: Color32,
    pub bg_menu: Color32,
    pub accent: Color32,
    pub fg: Color32,
    pub fg_muted: Color32,
    pub bordure: Color32,
    pub danger: Color32,
    pub clair: bool,
}

impl Palette {
    /// Couleur de texte lisible sur un fond de couleur d'accent.
    pub fn sur_accent(&self) -> Color32 {
        texte_sur(self.accent)
    }
}

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// Thème d'origine, repris de la version Python.
const OMNI: Palette = Palette {
    bg_main: rgb(0x121212),
    bg_btn: rgb(0x2a2a2a),
    bg_btn_hover: rgb(0x3a3a3a),
    bg_tuile_hover: rgb(0x1e1e1e),
    bg_menu: rgb(0x1a1a1a),
    accent: rgb(0xdeff9a),
    fg: Color32::WHITE,
    fg_muted: rgb(0x888888),
    bordure: rgb(0x4a4a4a),
    danger: rgb(0xff7a7a),
    clair: false,
};

const OLED_NOIR: Palette = Palette {
    bg_main: rgb(0x000000),
    bg_btn: rgb(0x1a1a1a),
    bg_btn_hover: rgb(0x2a2a2a),
    bg_tuile_hover: rgb(0x111111),
    bg_menu: rgb(0x0d0d0d),
    accent: rgb(0x4fc3f7),
    fg: rgb(0xf2f2f2),
    fg_muted: rgb(0x8a8a8a),
    bordure: rgb(0x333333),
    danger: rgb(0xff6b6b),
    clair: false,
};

const NORD: Palette = Palette {
    bg_main: rgb(0x2e3440),
    bg_btn: rgb(0x434c5e),
    bg_btn_hover: rgb(0x4c566a),
    bg_tuile_hover: rgb(0x3b4252),
    bg_menu: rgb(0x3b4252),
    accent: rgb(0x88c0d0),
    fg: rgb(0xeceff4),
    fg_muted: rgb(0xa3abbd),
    bordure: rgb(0x4c566a),
    danger: rgb(0xd57780),
    clair: false,
};

const DRACULA: Palette = Palette {
    bg_main: rgb(0x282a36),
    bg_btn: rgb(0x44475a),
    bg_btn_hover: rgb(0x565970),
    bg_tuile_hover: rgb(0x343746),
    bg_menu: rgb(0x21222c),
    accent: rgb(0xbd93f9),
    fg: rgb(0xf8f8f2),
    fg_muted: rgb(0x949bc4),
    bordure: rgb(0x5a5e78),
    danger: rgb(0xff5555),
    clair: false,
};

const CATPPUCCIN: Palette = Palette {
    bg_main: rgb(0x1e1e2e),
    bg_btn: rgb(0x313244),
    bg_btn_hover: rgb(0x45475a),
    bg_tuile_hover: rgb(0x2a2b3c),
    bg_menu: rgb(0x181825),
    accent: rgb(0xf5c2e7),
    fg: rgb(0xcdd6f4),
    fg_muted: rgb(0x9399b2),
    bordure: rgb(0x45475a),
    danger: rgb(0xf38ba8),
    clair: false,
};

const SYNTHWAVE: Palette = Palette {
    bg_main: rgb(0x1a1033),
    bg_btn: rgb(0x2d1f57),
    bg_btn_hover: rgb(0x3d2b73),
    bg_tuile_hover: rgb(0x26194a),
    bg_menu: rgb(0x211541),
    accent: rgb(0xff4fd8),
    fg: rgb(0xf4eaff),
    fg_muted: rgb(0xa592cc),
    bordure: rgb(0x4a3a80),
    danger: rgb(0xff6b6b),
    clair: false,
};

const CLAIR: Palette = Palette {
    bg_main: rgb(0xf4f5f7),
    bg_btn: rgb(0xe3e5ea),
    bg_btn_hover: rgb(0xd3d6dd),
    bg_tuile_hover: rgb(0xe6e8ec),
    bg_menu: rgb(0xffffff),
    accent: rgb(0x2f6fed),
    fg: rgb(0x1b1d22),
    fg_muted: rgb(0x5f6470),
    bordure: rgb(0xc5c9d2),
    danger: rgb(0xd03030),
    clair: true,
};

pub const TOUS: [Theme; 7] = [
    Theme::Omni,
    Theme::OledNoir,
    Theme::Nord,
    Theme::Dracula,
    Theme::Catppuccin,
    Theme::Synthwave,
    Theme::Clair,
];

pub fn nom(theme: Theme) -> &'static str {
    match theme {
        Theme::Omni => "Omni",
        Theme::OledNoir => "OLED noir",
        Theme::Nord => "Nord",
        Theme::Dracula => "Dracula",
        Theme::Catppuccin => "Catppuccin",
        Theme::Synthwave => "Synthwave",
        Theme::Clair => "Clair",
    }
}

pub fn palette_de(theme: Theme) -> Palette {
    match theme {
        Theme::Omni => OMNI,
        Theme::OledNoir => OLED_NOIR,
        Theme::Nord => NORD,
        Theme::Dracula => DRACULA,
        Theme::Catppuccin => CATPPUCCIN,
        Theme::Synthwave => SYNTHWAVE,
        Theme::Clair => CLAIR,
    }
}

static PALETTE: RwLock<Palette> = RwLock::new(OMNI);

/// Palette active.
pub fn p() -> Palette {
    *PALETTE.read().unwrap_or_else(|e| e.into_inner())
}

/// Noir ou blanc selon la luminance du fond.
pub fn texte_sur(fond: Color32) -> Color32 {
    let lum = 0.2126 * fond.r() as f32 + 0.7152 * fond.g() as f32 + 0.0722 * fond.b() as f32;
    if lum > 140.0 { Color32::BLACK } else { Color32::WHITE }
}

/// Famille de police grasse (Segoe UI Bold si disponible).
pub fn gras() -> FontFamily {
    FontFamily::Name("gras".into())
}

/// Polices et espacements : une seule fois au démarrage.
pub fn initialiser(ctx: &egui::Context) {
    charger_polices(ctx);
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

/// Applique un thème, avec éventuellement une couleur d'accent personnalisée.
pub fn appliquer(ctx: &egui::Context, theme: Theme, accent_perso: Option<[u8; 3]>) {
    let mut pal = palette_de(theme);
    if let Some([r, g, b]) = accent_perso {
        pal.accent = Color32::from_rgb(r, g, b);
    }
    *PALETTE.write().unwrap_or_else(|e| e.into_inner()) = pal;

    let mut v = if pal.clair {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };
    v.panel_fill = pal.bg_main;
    v.window_fill = pal.bg_menu;
    v.extreme_bg_color = pal.bg_btn;
    v.faint_bg_color = pal.bg_menu;
    v.window_stroke = Stroke::new(1.0, pal.bordure);
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(8);
    v.override_text_color = Some(pal.fg);
    v.selection.bg_fill = pal.accent.linear_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, pal.accent);
    v.hyperlink_color = pal.accent;

    let w = &mut v.widgets;
    w.inactive.weak_bg_fill = pal.bg_btn;
    w.inactive.bg_fill = pal.bg_btn;
    w.inactive.bg_stroke = Stroke::NONE;
    w.hovered.weak_bg_fill = pal.bg_btn_hover;
    w.hovered.bg_fill = pal.bg_btn_hover;
    w.hovered.bg_stroke = Stroke::new(1.0, pal.bordure);
    w.active.weak_bg_fill = pal.bg_btn_hover;
    w.active.bg_fill = pal.bg_btn_hover;
    w.active.bg_stroke = Stroke::new(1.0, pal.accent);
    for etat in [
        &mut w.inactive,
        &mut w.hovered,
        &mut w.active,
        &mut w.open,
        &mut w.noninteractive,
    ] {
        etat.corner_radius = CornerRadius::same(6);
    }
    w.noninteractive.bg_stroke = Stroke::new(1.0, pal.bordure);

    ctx.set_visuals(v);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texte_lisible_sur_accent() {
        for t in TOUS {
            let pal = palette_de(t);
            let fond = pal.accent;
            let texte = pal.sur_accent();
            assert_ne!(texte, fond, "thème {}", nom(t));
        }
        assert_eq!(texte_sur(Color32::from_rgb(0xde, 0xff, 0x9a)), Color32::BLACK);
        assert_eq!(texte_sur(Color32::from_rgb(0x2f, 0x6f, 0xed)), Color32::WHITE);
    }
}
