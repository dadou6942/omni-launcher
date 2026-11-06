//! Contenu de la bibliothèque : sections, filtres et grille virtualisée.

use super::*;

const ECART_TUILES: f32 = 14.0;

impl App {
    /// Jeux affichés, favoris en tête comme à l'écran.
    pub(super) fn ids_visibles(&self) -> Vec<u64> {
        let ids = self.bib.lister(Filtre {
            section: &self.section,
            recherche: &self.recherche,
            statut: self.filtre_statut,
        });
        if self.section == Section::Tout {
            let (favoris, autres) = self.bib.separer_favoris(&ids);
            favoris.into_iter().chain(autres).collect()
        } else {
            ids
        }
    }

    fn titre_page(&self) -> String {
        match &self.section {
            Section::Tout => "Bibliothèque".to_string(),
            Section::Recents => "Récents".to_string(),
            Section::Favoris => "Favoris".to_string(),
            Section::Collection(t) => t.clone(),
        }
    }

    /// Rangée de filtres par statut.
    fn filtres(&mut self, ui: &mut egui::Ui) {
        let statuts = self.bib.config.statuts();
        if self
            .filtre_statut
            .is_some_and(|f| !statuts.iter().any(|(s, _)| *s == f))
        {
            self.filtre_statut = None;
        }
        if statuts.is_empty() {
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(theme::ESPACE_S, theme::ESPACE_S);
            if puce(ui, "Tous", self.filtre_statut.is_none()).clicked() {
                self.filtre_statut = None;
            }
            for (statut, n) in statuts {
                let actif = self.filtre_statut == Some(statut);
                if puce(ui, &format!("{}  {n}", statut.libelle()), actif).clicked() {
                    self.filtre_statut = if actif { None } else { Some(statut) };
                }
            }
        });
        ui.add_space(theme::ESPACE_L);
    }

    fn bibliotheque_vide(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let p = theme::p();
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.25);
            ui.label(
                RichText::new("Aucun jeu dans votre bibliothèque")
                    .size(theme::POLICE_TITRE)
                    .color(p.fg),
            );
            ui.add_space(theme::ESPACE_S);
            ui.label(
                RichText::new("Ajoutez un exécutable, scannez votre dossier de jeux, ou glissez-déposez-les ici.")
                    .color(p.fg_muted),
            );
            ui.add_space(theme::ESPACE_L);
            ui.horizontal(|ui| {
                ui.add_space(((ui.available_width() - 400.0) / 2.0).max(0.0));
                let ajouter = egui::Button::new(RichText::new("+  Ajouter un jeu").color(p.sur_accent()))
                    .fill(p.accent);
                if ui.add(ajouter).clicked() {
                    actions.push(Action::Ajouter);
                }
                let scan = egui::Button::new("🗁  Scanner un dossier");
                if ui.add_enabled(self.scan.is_none(), scan).clicked() {
                    actions.push(Action::Scanner);
                }
            });
        });
    }

    pub(super) fn bibliotheque(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let p = theme::p();
        if self.bib.config.jeux.is_empty() {
            self.bibliotheque_vide(ui, actions);
            return;
        }

        let ids = self.ids_visibles();
        let recherche = !self.recherche.trim().is_empty();
        let filtre_actif = recherche || self.filtre_statut.is_some();
        let accueil = self.section == Section::Tout && !filtre_actif;

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let marge = egui::Margin::symmetric(theme::ESPACE_XL as i8, 0);
            egui::Frame::new().inner_margin(marge).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(self.titre_page());
                    ui.label(RichText::new(format!("{} jeux", ids.len())).color(p.fg_muted));
                });
                ui.add_space(theme::ESPACE_M);
                self.filtres(ui);

                if accueil && self.bib.config.afficher_recents {
                    self.recents(ui, actions);
                }

                if ids.is_empty() {
                    ui.add_space(theme::ESPACE_XXL);
                    ui.vertical_centered(|ui| {
                        let texte = if recherche {
                            format!("Aucun jeu ne correspond à « {} ».", self.recherche.trim())
                        } else {
                            "Aucun jeu ici pour le moment.".to_string()
                        };
                        ui.label(RichText::new(texte).color(p.fg_muted));
                    });
                } else if self.section == Section::Tout {
                    let (favoris, autres) = self.bib.separer_favoris(&ids);
                    self.sections_accueil(ui, &favoris, &autres, accueil, recherche, actions);
                } else {
                    self.afficher_jeux(ui, &ids, actions);
                }
                ui.add_space(theme::ESPACE_XL);
            });
        });
    }

    fn sections_accueil(
        &mut self,
        ui: &mut egui::Ui,
        favoris: &[u64],
        autres: &[u64],
        accueil: bool,
        recherche: bool,
        actions: &mut Vec<Action>,
    ) {
        let p = theme::p();
        if accueil || !favoris.is_empty() {
            titre_section(ui, "Vos favoris ⭐");
            if favoris.is_empty() {
                ui.label(
                    RichText::new("Aucun favori pour le moment. Clic droit sur un jeu ➔ Ajouter aux favoris.")
                        .italics()
                        .color(p.fg_muted),
                );
                ui.add_space(theme::ESPACE_L);
            } else {
                self.afficher_jeux(ui, favoris, actions);
            }
        }
        if !autres.is_empty() {
            ui.add_space(theme::ESPACE_M);
            let titre = match self.filtre_statut {
                _ if recherche => "Résultats",
                Some(s) => s.libelle(),
                None => "Autres jeux",
            };
            titre_section(ui, titre);
            self.afficher_jeux(ui, autres, actions);
        }
    }

    fn afficher_jeux(&mut self, ui: &mut egui::Ui, ids: &[u64], actions: &mut Vec<Action>) {
        match self.bib.config.affichage {
            Affichage::Grille | Affichage::Jaquettes => self.grille(ui, ids, actions),
            Affichage::Liste => self.liste(ui, ids, actions),
        }
    }

    /// Seules les rangées visibles sont construites : la place des autres est
    /// réservée, ce qui garde un défilement fluide sur une grande bibliothèque.
    fn grille(&mut self, ui: &mut egui::Ui, ids: &[u64], actions: &mut Vec<Action>) {
        let cote = self.bib.config.taille_tuile;
        let hauteur = match self.bib.config.affichage {
            Affichage::Jaquettes => cote * RATIO_JAQUETTE,
            Affichage::Grille | Affichage::Liste => cote,
        };
        let taille = Vec2::new(cote + 24.0, hauteur + 56.0);
        let largeur = ui.available_width();
        let colonnes =
            (((largeur + ECART_TUILES) / (taille.x + ECART_TUILES)).floor() as usize).max(1);

        for ligne in ids.chunks(colonnes) {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(largeur, taille.y), Sense::hover());
            if ui.is_rect_visible(rect) {
                let cadre = egui::UiBuilder::new()
                    .id_salt(("rangee", ligne[0]))
                    .max_rect(rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Min));
                let mut rangee = ui.new_child(cadre);
                rangee.spacing_mut().item_spacing.x = ECART_TUILES;
                for &id in ligne {
                    self.tuile(&mut rangee, id, taille, actions);
                }
            }
            ui.add_space(ECART_TUILES);
        }
    }
}
