//! Vue Paramètres : affichage, apparence, bibliothèque, images, tags,
//! sauvegardes, mises à jour.

use super::*;

impl App {
    pub(super) fn parametres(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let p = theme::p();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.set_max_width(680.0);
                ui.add_space(10.0);
                ui.label(RichText::new("Configuration du launcher").size(26.0).family(theme::gras()));
                ui.add_space(24.0);

                carte(ui, "Affichage", |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().slider_width = 260.0;
                        ui.label("Taille des icônes");
                        let r = ui.add(
                            egui::Slider::new(&mut self.bib.config.taille_tuile, TAILLE_TUILE_MIN..=TAILLE_TUILE_MAX)
                                .step_by(5.0)
                                .suffix(" px"),
                        );
                        if r.changed() {
                            self.sauvegarde_differee = true;
                        }
                    });
                    if ui
                        .checkbox(&mut self.bib.config.afficher_recents, "Afficher la section « Joués récemment »")
                        .changed()
                    {
                        self.sauver();
                    }
                    if ui
                        .checkbox(
                            &mut self.bib.config.fermer_apres_lancement,
                            "Fermer le launcher à la fin de la partie",
                        )
                        .changed()
                    {
                        self.sauver();
                    }
                    ui.label(
                        RichText::new(
                            "Pendant une partie, le launcher se réduit pour mesurer le temps de jeu. \
                             Décochez pour qu'il réapparaisse à la fin.",
                        )
                        .small()
                        .color(p.fg_muted),
                    );
                });

                carte(ui, "Apparence", |ui| self.carte_apparence(ui, &ctx));

                carte(ui, "Plein écran et manette", |ui| self.carte_plein_ecran(ui));

                carte(ui, "Bibliothèque", |ui| {
                    let favoris = self.bib.config.jeux.iter().filter(|j| j.favori).count();
                    ui.label(format!("{} jeux, dont {} favoris.", self.bib.config.jeux.len(), favoris));
                    if let Some(d) = &self.bib.config.dernier_dossier_scan {
                        ui.label(RichText::new(format!("Dernier dossier scanné : {}", d.display())).color(p.fg_muted));
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui.add_enabled(self.scan.is_none(), egui::Button::new("🗁  Scanner un dossier…")).clicked() {
                            self.choisir_dossier_scan(&ctx);
                        }
                        if let Some(d) = self.bib.config.dernier_dossier_scan.clone()
                            && ui
                                .add_enabled(self.scan.is_none(), egui::Button::new("🔄  Re-scanner"))
                                .on_hover_text("Ajoute les jeux installés depuis le dernier scan")
                                .clicked()
                            {
                                self.lancer_scan(d, &ctx);
                            }
                    });
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Vérifier les jeux introuvables").clicked() {
                            self.bib.verifier_manquants();
                            let n = self.bib.manquants.len();
                            self.notifier(match n {
                                0 => "Tous les jeux sont présents.".to_string(),
                                n => format!("{n} jeu(x) introuvable(s)."),
                            });
                        }
                        let n = self.bib.manquants.len();
                        if ui
                            .add_enabled(n > 0, egui::Button::new(format!("Retirer les introuvables ({n})")))
                            .clicked()
                        {
                            self.dialogue = Some(Dialogue::RetirerIntrouvables);
                        }
                    });
                });

                carte(ui, "Jaquettes et bannières", |ui| self.carte_visuels(ui, &ctx));

                carte(ui, "Tags", |ui| self.carte_tags(ui));

                carte(ui, "Sauvegardes", |ui| self.carte_sauvegardes(ui));

                carte(ui, "Mises à jour", |ui| self.carte_maj(ui, &ctx));

                carte(ui, "Raccourcis clavier", palette::tableau_raccourcis);

                carte(ui, "Données", |ui| {
                    ui.label(RichText::new("Configuration, icônes personnalisées, sauvegardes et cache :").color(p.fg_muted));
                    ui.label(RichText::new(self.bib.dossiers.racine.display().to_string()).color(p.accent).strong());
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("🗁  Ouvrir le dossier").clicked() {
                            let racine = self.bib.dossiers.racine.clone();
                            if let Err(e) = platform::ouvrir_dossier(&racine) {
                                self.notifier_erreur(e);
                            }
                        }
                        if ui.button("Vider le cache d'icônes").clicked() {
                            self.vider_cache();
                        }
                    });
                });

                carte(ui, "Astuces", |ui| {
                    for astuce in [
                        "Double-clic sur un jeu pour le lancer, clic droit pour les options.",
                        "Le menu à côté du tri passe des icônes aux jaquettes ou à la liste.",
                        "Ctrl+F pour rechercher (nom ou tag), Entrée lance le premier résultat, Échap efface.",
                        "F11 ou le bouton Start de la manette ouvre le mode plein écran.",
                        "Glissez-déposez des .exe ou un dossier de jeux directement dans la fenêtre.",
                        "« Supprimer le jeu du disque » envoie son dossier à la corbeille : c'est récupérable.",
                    ] {
                        ui.label(RichText::new(format!("•  {astuce}")).color(p.fg_muted));
                    }
                });
            });
        });
    }

    fn carte_apparence(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let p = theme::p();
        ui.label("Thème");
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
            for t in theme::TOUS {
                if apercu_theme(ui, t, self.bib.config.theme == t).clicked() && self.bib.config.theme != t {
                    self.bib.config.theme = t;
                    // Un nouveau thème vient avec sa propre couleur d'accent.
                    self.bib.config.accent_perso = None;
                    self.appliquer_theme(ctx);
                    self.sauver();
                }
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Couleur d'accent");
            let a = p.accent;
            let mut couleur = [a.r(), a.g(), a.b()];
            if egui::color_picker::color_edit_button_srgb(ui, &mut couleur).changed() {
                self.bib.config.accent_perso = Some(couleur);
                self.appliquer_theme(ctx);
                self.sauvegarde_differee = true;
            }
            if self.bib.config.accent_perso.is_some() && ui.button("Couleur du thème").clicked() {
                self.bib.config.accent_perso = None;
                self.appliquer_theme(ctx);
                self.sauver();
            }
        });
    }

    fn carte_plein_ecran(&mut self, ui: &mut egui::Ui) {
        use crate::config::{Glyphes, StylePleinEcran};
        let avant = (self.bib.config.style_plein_ecran, self.bib.config.glyphes);
        let c = &mut self.bib.config;
        ui.horizontal(|ui| {
            ui.label("Style");
            ui.selectable_value(&mut c.style_plein_ecran, StylePleinEcran::Xmb, "Menu XMB");
            ui.selectable_value(&mut c.style_plein_ecran, StylePleinEcran::Grille, "Grille");
        });
        ui.horizontal(|ui| {
            ui.label("Boutons affichés");
            ui.selectable_value(&mut c.glyphes, Glyphes::Auto, "Automatique");
            ui.selectable_value(&mut c.glyphes, Glyphes::Xbox, "Xbox");
            ui.selectable_value(&mut c.glyphes, Glyphes::PlayStation, "PlayStation");
        });
        let mut change = avant != (c.style_plein_ecran, c.glyphes);
        for (champ, libelle) in [
            (&mut c.fond_anime, "Fond animé (à désactiver si la machine est lente)"),
            (&mut c.sons, "Sons de navigation"),
            (&mut c.demarrer_plein_ecran, "Démarrer en plein écran"),
            (&mut c.bascule_manette, "Passer en plein écran quand une manette est branchée"),
        ] {
            change |= ui.checkbox(champ, libelle).changed();
        }
        if change {
            self.sauver();
        }
    }

    fn carte_tags(&mut self, ui: &mut egui::Ui) {
        let p = theme::p();
        let tags = self.bib.config.tags();
        if tags.is_empty() {
            ui.label(
                RichText::new("Aucun tag. Clic droit sur un jeu ➔ Tags… pour en créer (ex. Coop, Rétro, À finir).")
                    .color(p.fg_muted),
            );
            return;
        }
        ui.label(
            RichText::new("Les tags servent à filtrer la bibliothèque et le mode plein écran.")
                .color(p.fg_muted),
        );
        for (tag, n) in tags {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&tag).strong());
                ui.label(RichText::new(format!("{n} jeu(x)")).color(p.fg_muted));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(RichText::new("Supprimer").color(p.danger)).clicked() {
                        self.dialogue = Some(Dialogue::SupprimerTag(tag.clone()));
                    }
                    if ui.button("Renommer").clicked() {
                        self.dialogue = Some(Dialogue::RenommerTag {
                            texte: tag.clone(),
                            ancien: tag.clone(),
                        });
                    }
                });
            });
        }
    }

    fn carte_sauvegardes(&mut self, ui: &mut egui::Ui) {
        let p = theme::p();
        ui.label(
            RichText::new(format!(
                "Une copie automatique est faite chaque jour au démarrage ; les {} plus récentes sont conservées.",
                sauvegarde::SAUVEGARDES_MAX
            ))
            .color(p.fg_muted),
        );
        ui.horizontal_wrapped(|ui| {
            if ui.button("⬆  Exporter la bibliothèque…").clicked() {
                self.exporter();
            }
            if ui.button("⬇  Importer…").clicked() {
                self.choisir_import();
            }
            if ui.button("🗁  Ouvrir le dossier").clicked() {
                let d = self.bib.dossiers.sauvegardes.clone();
                if let Err(e) = platform::ouvrir_dossier(&d) {
                    self.notifier_erreur(e);
                }
            }
        });
        ui.add_space(6.0);
        if self.sauvegardes.is_empty() {
            ui.label(RichText::new("Aucune sauvegarde pour l'instant.").italics().color(p.fg_muted));
            return;
        }
        let mut restaurer = None;
        for info in &self.sauvegardes {
            ui.horizontal(|ui| {
                ui.label(sauvegarde::libelle_date(info.date));
                let detail = match info.nb_jeux {
                    Some(n) => format!("{n} jeu(x)"),
                    None => "fichier illisible".into(),
                };
                ui.label(RichText::new(detail).color(p.fg_muted));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(info.nb_jeux.is_some(), egui::Button::new("Restaurer"))
                        .clicked()
                    {
                        restaurer = Some(info.clone());
                    }
                });
            });
        }
        if let Some(info) = restaurer {
            self.dialogue = Some(Dialogue::Restaurer(info));
        }
    }
}

/// Vignette cliquable montrant les couleurs d'un thème.
fn apercu_theme(ui: &mut egui::Ui, t: crate::config::Theme, actif: bool) -> egui::Response {
    let pal = theme::palette_de(t);
    let courant = theme::p();
    let (rect, reponse) = ui.allocate_exact_size(Vec2::new(118.0, 70.0), Sense::click());
    let peintre = ui.painter();
    peintre.rect_filled(rect, CornerRadius::same(8), pal.bg_main);
    let contour = if actif {
        Stroke::new(2.5, courant.accent)
    } else if reponse.hovered() {
        Stroke::new(1.5, courant.fg_muted)
    } else {
        Stroke::new(1.0, courant.bordure)
    };
    peintre.rect_stroke(rect, CornerRadius::same(8), contour, egui::StrokeKind::Inside);

    // Une « tuile » et un bouton miniatures.
    let tuile = egui::Rect::from_min_size(rect.min + Vec2::new(10.0, 10.0), Vec2::new(26.0, 26.0));
    peintre.rect_filled(tuile, CornerRadius::same(5), pal.bg_btn);
    let bouton = egui::Rect::from_min_size(rect.min + Vec2::new(44.0, 16.0), Vec2::new(36.0, 14.0));
    peintre.rect_filled(bouton, CornerRadius::same(7), pal.accent);
    peintre.circle_filled(rect.min + Vec2::new(96.0, 23.0), 6.0, pal.fg_muted);
    peintre.text(
        rect.left_bottom() + Vec2::new(10.0, -10.0),
        Align2::LEFT_BOTTOM,
        theme::nom(t),
        FontId::new(13.0, theme::gras()),
        pal.fg,
    );
    reponse.on_hover_cursor(egui::CursorIcon::PointingHand)
}
