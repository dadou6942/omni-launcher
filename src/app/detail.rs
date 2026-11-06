//! Panneau de détail du jeu sélectionné, à droite de la bibliothèque.

use super::*;

const LARGEUR_DETAIL: f32 = 340.0;
const HAUTEUR_BANNIERE: f32 = 170.0;

impl App {
    /// Image du haut du panneau : bannière, sinon jaquette, sinon icône.
    fn entete_detail(&mut self, ui: &mut egui::Ui, jeu: &Jeu) {
        let p = theme::p();
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(LARGEUR_DETAIL, HAUTEUR_BANNIERE),
            Sense::hover(),
        );
        ui.painter().rect_filled(rect, CornerRadius::ZERO, p.bg_btn);
        let banniere = jeu
            .banniere
            .as_deref()
            .and_then(|f| self.icones.visuel(jeu.id, Genre::Banniere, f));
        let jaquette = jeu
            .jaquette
            .as_deref()
            .and_then(|f| self.icones.visuel(jeu.id, Genre::Jaquette, f));
        match banniere.or(jaquette) {
            Some(t) => {
                let uv = icons::uv_couvrir(t.size_vec2(), rect.size());
                egui::Image::new(&t).uv(uv).paint_at(ui, rect);
            }
            None => {
                let zone = egui::Rect::from_center_size(rect.center(), Vec2::splat(112.0));
                self.dessiner_icone(ui, jeu, zone, self.bib.manquants.contains(&jeu.id));
            }
        }
    }

    fn corps_detail(&mut self, ui: &mut egui::Ui, jeu: &Jeu, actions: &mut Vec<Action>) {
        let p = theme::p();
        let id = jeu.id;
        ui.add_space(theme::ESPACE_L);
        ui.label(RichText::new(&jeu.nom).size(22.0).family(theme::gras()));
        ui.label(RichText::new(resume_jeu(jeu)).color(p.fg_muted));
        if self.bib.manquants.contains(&id) {
            ui.label(RichText::new("Exécutable introuvable").color(p.danger));
        }
        ui.add_space(theme::ESPACE_M);

        let largeur = ui.available_width();
        let lancer = egui::Button::new(RichText::new("▶  Lancer").color(p.sur_accent()).strong())
            .fill(p.accent)
            .min_size(Vec2::new(largeur, 44.0))
            .corner_radius(CornerRadius::same(theme::RAYON_M));
        if ui.add(lancer).clicked() {
            actions.push(Action::Lancer(id));
        }
        ui.add_space(theme::ESPACE_XS);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = theme::ESPACE_S;
            let favori = if jeu.favori {
                RichText::new("⭐  Favori").color(p.accent)
            } else {
                RichText::new("☆  Favori")
            };
            if ui.button(favori).clicked() {
                actions.push(Action::BasculerFavori(id));
            }
            if ui.button("🗁  Dossier").clicked() {
                actions.push(Action::OuvrirEmplacement(id));
            }
            ui.menu_button("✏  Modifier", |ui| {
                ui.set_min_width(220.0);
                entrees_modifier(ui, jeu, actions);
            });
        });

        ui.add_space(theme::ESPACE_M);
        if jeu.statut != Statut::Aucun {
            ui.label(RichText::new(jeu.statut.libelle()).color(couleur_statut(jeu.statut)));
        }
        if !jeu.tags.is_empty() {
            ui.label(RichText::new(format!("🏷 {}", jeu.tags.join("  ·  "))).color(p.fg_muted));
        }
        if !jeu.notes.is_empty() {
            ui.add_space(theme::ESPACE_S);
            ui.label(RichText::new("Notes").strong());
            ui.label(RichText::new(&jeu.notes).color(p.fg_muted));
        }
        ui.add_space(theme::ESPACE_S);
        ui.add(
            egui::Label::new(
                RichText::new(jeu.exe.display().to_string())
                    .small()
                    .color(p.fg_muted),
            )
            .wrap(),
        );

        ui.add_space(theme::ESPACE_L);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = theme::ESPACE_S;
            if ui.button("➖  Retirer").clicked() {
                actions.push(Action::DemanderRetrait(id));
            }
            if ui
                .button(RichText::new("🗑  Supprimer du disque").color(p.danger))
                .clicked()
            {
                actions.push(Action::DemanderSuppression(id));
            }
        });
    }

    pub(super) fn panneau_detail(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let p = theme::p();
        if self.selection.is_some_and(|id| self.bib.jeu(id).is_none()) {
            self.selection = None;
        }
        if self.selection.is_some() {
            self.detail_affiche = self.selection;
        }
        let ouverture = ui.ctx().animate_bool_with_time(
            egui::Id::unique("ouverture_detail"),
            self.selection.is_some(),
            theme::DUREE_NORMALE,
        );
        let Some(jeu) = self.detail_affiche.and_then(|id| self.bib.jeu(id)).cloned() else {
            return;
        };
        if ouverture <= 0.0 {
            return;
        }

        egui::Panel::right("panneau_detail")
            .exact_size(LARGEUR_DETAIL * ouverture)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::new().fill(p.bg_menu))
            .show(ui, |ui| {
                ui.set_min_width(LARGEUR_DETAIL);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.entete_detail(ui, &jeu);
                        egui::Frame::new()
                            .inner_margin(egui::Margin::same(theme::ESPACE_L as i8))
                            .show(ui, |ui| {
                                ui.set_max_width(LARGEUR_DETAIL - 2.0 * theme::ESPACE_L);
                                self.corps_detail(ui, &jeu, actions);
                            });
                    });
                let fermer = egui::Rect::from_min_size(
                    ui.max_rect().right_top() + Vec2::new(-40.0, 8.0),
                    Vec2::splat(32.0),
                );
                if ui
                    .put(fermer, egui::Button::new("×"))
                    .on_hover_text("Fermer le panneau (Échap)")
                    .clicked()
                {
                    actions.push(Action::FermerDetail);
                }
            });
    }
}
