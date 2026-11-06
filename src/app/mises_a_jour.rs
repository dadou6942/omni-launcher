//! Recherche d'une nouvelle version et son affichage. La recherche se fait en
//! arrière-plan ; au démarrage, un échec reste silencieux (pas de réseau, par
//! exemple).

use super::*;
use crate::maj::{self, Publication};

type ResultatMaj = Result<Option<Publication>, String>;

pub(super) enum EtatMaj {
    Inconnu,
    Recherche {
        manuelle: bool,
        reception: Receiver<ResultatMaj>,
    },
    AJour,
    Disponible(Publication),
    Erreur(String),
}

impl App {
    pub(super) fn chercher_maj(&mut self, ctx: &egui::Context, manuelle: bool) {
        if matches!(self.maj, EtatMaj::Recherche { .. }) {
            return;
        }
        let (tx, reception) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(maj::verifier());
            ctx.request_repaint();
        });
        self.maj = EtatMaj::Recherche { manuelle, reception };
    }

    pub(super) fn suivre_maj(&mut self) {
        let EtatMaj::Recherche { manuelle, reception } = &self.maj else {
            return;
        };
        let manuelle = *manuelle;
        let resultat = match reception.try_recv() {
            Ok(r) => r,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Err("La recherche s'est interrompue.".to_string())
            }
        };
        self.maj = match resultat {
            Ok(Some(publication)) => EtatMaj::Disponible(publication),
            Ok(None) => EtatMaj::AJour,
            Err(e) if manuelle => EtatMaj::Erreur(e),
            Err(_) => EtatMaj::Inconnu,
        };
    }

    fn ouvrir_page_maj(&mut self, url: &str) {
        if let Err(e) = platform::ouvrir_url(url) {
            self.notifier_erreur(e);
        }
    }

    /// Bouton de l'en-tête, visible seulement si une version est disponible.
    pub(super) fn bouton_maj(&mut self, ui: &mut egui::Ui) {
        let EtatMaj::Disponible(publication) = &self.maj else {
            return;
        };
        let p = theme::p();
        let url = publication.url.clone();
        let texte = format!("⬆  Version {} disponible", publication.version);
        if ui
            .button(RichText::new(texte).color(p.accent))
            .on_hover_text("Ouvre la page de téléchargement sur GitHub")
            .clicked()
        {
            self.ouvrir_page_maj(&url);
        }
    }

    pub(super) fn carte_maj(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let p = theme::p();
        ui.label(format!("Version installée : {}", maj::VERSION_ACTUELLE));
        match &self.maj {
            EtatMaj::Inconnu => {}
            EtatMaj::Recherche { .. } => {
                ui.label(RichText::new("Recherche en cours…").color(p.fg_muted));
            }
            EtatMaj::AJour => {
                ui.label(RichText::new("Le launcher est à jour.").color(p.fg_muted));
            }
            EtatMaj::Disponible(publication) => {
                ui.label(
                    RichText::new(format!("Version {} disponible.", publication.version))
                        .color(p.accent)
                        .strong(),
                );
            }
            EtatMaj::Erreur(e) => {
                ui.label(RichText::new(e.as_str()).color(p.danger));
            }
        }
        let url = match &self.maj {
            EtatMaj::Disponible(publication) => Some(publication.url.clone()),
            _ => None,
        };
        ui.horizontal_wrapped(|ui| {
            let recherche = matches!(self.maj, EtatMaj::Recherche { .. });
            if ui
                .add_enabled(!recherche, egui::Button::new("🔄  Vérifier maintenant"))
                .clicked()
            {
                self.chercher_maj(ctx, true);
            }
            if let Some(url) = url
                && ui.button("Ouvrir la page de téléchargement").clicked()
            {
                self.ouvrir_page_maj(&url);
            }
        });
        if ui
            .checkbox(&mut self.bib.config.verifier_maj, "Vérifier au démarrage")
            .changed()
        {
            self.sauver();
        }
        ui.label(
            RichText::new(
                "Le launcher interroge GitHub, ne télécharge rien et n'installe rien de lui-même.",
            )
            .small()
            .color(p.fg_muted),
        );
    }
}
