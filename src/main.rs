// Pas de console noire au lancement sous Windows (équivalent de --noconsole).
#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod bibliotheque;
mod config;
mod icons;
mod maj;
mod manette;
mod partie;
mod platform;
mod reseau;
mod sauvegarde;
mod scan;
mod sons;
mod stats;
mod steamgriddb;
mod theme;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Omni Launcher")
        .with_inner_size([1200.0, 800.0])
        .with_min_inner_size([640.0, 420.0])
        .with_drag_and_drop(true);

    if let Some(icone) = charger_icone_fenetre() {
        viewport = viewport.with_icon(icone);
    }

    let options = eframe::NativeOptions {
        viewport,
        // Mémorise taille et position de la fenêtre entre deux sessions.
        persist_window: true,
        ..Default::default()
    };

    eframe::run_native(
        "Omni Launcher",
        options,
        Box::new(|cc| {
            theme::initialiser(&cc.egui_ctx);
            Ok(Box::new(app::App::nouveau(&cc.egui_ctx)))
        }),
    )
}

/// Icône de la fenêtre : celle intégrée à la compilation, sinon un
/// `omni_launcher.ico` posé à côté de l'exécutable.
fn charger_icone_fenetre() -> Option<std::sync::Arc<egui::IconData>> {
    #[cfg(icone_integree)]
    let octets: Option<Vec<u8>> = Some(include_bytes!("../omni_launcher.ico").to_vec());
    #[cfg(not(icone_integree))]
    let octets: Option<Vec<u8>> = None;

    let octets = match octets {
        Some(o) => o,
        None => std::fs::read(
            std::env::current_exe()
                .ok()?
                .parent()?
                .join("omni_launcher.ico"),
        )
        .ok()?,
    };
    let img = image::load_from_memory(&octets).ok()?.to_rgba8();
    let (width, height) = img.dimensions();
    Some(std::sync::Arc::new(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    }))
}
