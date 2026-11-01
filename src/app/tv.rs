//! Mode plein écran, pensé pour la manette (ou les flèches du clavier) :
//! grandes tuiles, une sélection, des filtres qu'on fait défiler avec LB/RB.

use super::*;

const ECART: f32 = 22.0;

#[derive(Default)]
pub(super) struct EtatTv {
    selection: Option<u64>,
    /// Index dans la liste des filtres (0 = Tous, 1 = Favoris, puis les tags).
    filtre: usize,
    /// Nombre de colonnes de la dernière image, pour la navigation verticale.
    colonnes: usize,
    /// Faire défiler la grille jusqu'à la sélection à la prochaine image.
    defiler: bool,
}

impl EtatTv {
    pub(super) fn entrer(&mut self) {
        self.defiler = true;
    }
}

#[derive(Clone, PartialEq)]
enum FiltreTv {
    Tous,
    Favoris,
    Tag(String),
}

impl FiltreTv {
    fn libelle(&self) -> String {
        match self {
            FiltreTv::Tous => "Tous".into(),
            FiltreTv::Favoris => "⭐ Favoris".into(),
            FiltreTv::Tag(t) => t.clone(),
        }
    }
}

/// Nouvel index après un déplacement dans une grille de `colonnes` colonnes.
fn deplacer(i: usize, n: usize, colonnes: usize, c: Commande) -> usize {
    if n == 0 {
        return 0;
    }
    let col = colonnes.max(1);
    let i = i.min(n - 1);
    match c {
        Commande::Gauche => i.saturating_sub(1),
        Commande::Droite => (i + 1).min(n - 1),
        Commande::Haut if i >= col => i - col,
        Commande::Bas if i + col < n => i + col,
        // Dernière ligne incomplète : on descend sur le dernier jeu.
        Commande::Bas if i / col < (n - 1) / col => n - 1,
        _ => i,
    }
}

impl App {
    fn filtres_tv(&self) -> Vec<FiltreTv> {
        let mut f = vec![FiltreTv::Tous, FiltreTv::Favoris];
        f.extend(self.config.tags().into_iter().map(|(t, _)| FiltreTv::Tag(t)));
        f
    }

    fn filtre_tv(&self) -> FiltreTv {
        let filtres = self.filtres_tv();
        filtres
            .get(self.tv.filtre)
            .cloned()
            .unwrap_or(FiltreTv::Tous)
    }

    fn ids_tv(&self) -> Vec<u64> {
        let (fav, autres) = match self.filtre_tv() {
            FiltreTv::Tous => self.trier_filtrer("", None),
            FiltreTv::Favoris => (self.trier_filtrer("", None).0, Vec::new()),
            FiltreTv::Tag(t) => self.trier_filtrer("", Some(&t)),
        };
        fav.into_iter().chain(autres).collect()
    }

    /// Traduit le clavier en commandes de manette.
    pub(super) fn clavier_tv(&mut self, ctx: &egui::Context) {
        let correspondances = [
            (Key::ArrowUp, Commande::Haut),
            (Key::ArrowDown, Commande::Bas),
            (Key::ArrowLeft, Commande::Gauche),
            (Key::ArrowRight, Commande::Droite),
            (Key::Enter, Commande::Valider),
            (Key::Space, Commande::Valider),
            (Key::Escape, Commande::Retour),
            (Key::Backspace, Commande::Retour),
            (Key::F, Commande::Favori),
            (Key::Q, Commande::FiltrePrecedent),
            (Key::PageUp, Commande::FiltrePrecedent),
            (Key::E, Commande::FiltreSuivant),
            (Key::PageDown, Commande::FiltreSuivant),
        ];
        let commandes: Vec<Commande> = ctx.input(|i| {
            if i.modifiers.command || i.modifiers.alt {
                return Vec::new();
            }
            correspondances
                .iter()
                .filter(|(k, _)| i.key_pressed(*k))
                .map(|(_, c)| *c)
                .collect()
        });
        self.commandes.extend(commandes);
    }

    pub(super) fn commande_tv(&mut self, c: Commande, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let ids = self.ids_tv();
        let i = self
            .tv
            .selection
            .and_then(|s| ids.iter().position(|&id| id == s))
            .unwrap_or(0);
        match c {
            Commande::Haut | Commande::Bas | Commande::Gauche | Commande::Droite => {
                if let Some(&id) = ids.get(deplacer(i, ids.len(), self.tv.colonnes, c)) {
                    self.tv.selection = Some(id);
                    self.tv.defiler = true;
                }
            }
            Commande::Valider => {
                if let Some(&id) = ids.get(i) {
                    actions.push(Action::Lancer(id));
                }
            }
            Commande::Favori => {
                if let Some(&id) = ids.get(i) {
                    actions.push(Action::BasculerFavori(id));
                    self.tv.defiler = true;
                }
            }
            Commande::FiltrePrecedent | Commande::FiltreSuivant => {
                let n = self.filtres_tv().len();
                self.tv.filtre = if c == Commande::FiltreSuivant {
                    (self.tv.filtre + 1) % n
                } else {
                    (self.tv.filtre + n - 1) % n
                };
                self.tv.selection = self.ids_tv().first().copied();
                self.tv.defiler = true;
            }
            Commande::Retour => self.basculer_mode_tv(ctx),
            Commande::BasculerPleinEcran => {}
        }
    }

    pub(super) fn vue_tv(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let p = theme::p();
        let ctx = ui.ctx().clone();
        // L'horloge de l'en-tête.
        ctx.request_repaint_after(Duration::from_secs(20));

        if self.tv.filtre >= self.filtres_tv().len() {
            self.tv.filtre = 0;
        }
        let ids = self.ids_tv();
        if self.tv.selection.is_none_or(|s| !ids.contains(&s)) {
            self.tv.selection = ids.first().copied();
        }

        egui::Panel::top("tv_haut")
            .frame(egui::Frame::new().fill(p.bg_main).inner_margin(egui::Margin::symmetric(40, 24)))
            .show(ui, |ui| self.tv_entete(ui));

        egui::Panel::bottom("tv_bas")
            .frame(
                egui::Frame::new()
                    .fill(p.bg_menu)
                    .inner_margin(egui::Margin::symmetric(40, 18)),
            )
            .show(ui, |ui| self.tv_details(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg_main))
            .show(ui, |ui| self.tv_grille(ui, &ids, actions));
    }

    fn tv_entete(&mut self, ui: &mut egui::Ui) {
        let p = theme::p();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Omni Launcher").size(30.0).family(theme::gras()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(chrono::Local::now().format("%H:%M").to_string())
                        .size(26.0)
                        .family(theme::gras()),
                );
                ui.add_space(16.0);
                let (icone, texte) = match &self.manette_connectee {
                    Some(nom) => ("🎮", nom.clone()),
                    None => ("⌨", "Clavier".to_string()),
                };
                ui.label(RichText::new(format!("{icone}  {texte}")).color(p.fg_muted));
            });
        });
        ui.add_space(16.0);

        // Filtres : LB / RB pour passer de l'un à l'autre.
        let filtres = self.filtres_tv();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            badge(ui, if self.manette_connectee.is_some() { "LB" } else { "Q" });
            for (i, f) in filtres.iter().enumerate() {
                let r = ui.add(
                    egui::Button::new(
                        RichText::new(f.libelle())
                            .size(17.0)
                            .color(if i == self.tv.filtre { p.sur_accent() } else { p.fg }),
                    )
                    .fill(if i == self.tv.filtre { p.accent } else { p.bg_btn })
                    .corner_radius(CornerRadius::same(18))
                    .min_size(Vec2::new(0.0, 36.0)),
                );
                if r.clicked() {
                    self.tv.filtre = i;
                    self.tv.selection = None;
                    self.tv.defiler = true;
                }
            }
            badge(ui, if self.manette_connectee.is_some() { "RB" } else { "E" });
        });
    }

    fn tv_grille(&mut self, ui: &mut egui::Ui, ids: &[u64], actions: &mut Vec<Action>) {
        let p = theme::p();
        if ids.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.3);
                let texte = if self.config.jeux.is_empty() {
                    "Aucun jeu dans la bibliothèque."
                } else {
                    "Aucun jeu dans ce filtre."
                };
                ui.label(RichText::new(texte).size(22.0).color(p.fg_muted));
            });
            return;
        }

        let cote = (ui.available_height() / 2.6).clamp(140.0, 240.0);
        let taille = Vec2::new(cote + 28.0, cote + 52.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(12.0);
                let largeur = ui.available_width() - 80.0;
                let colonnes = (((largeur + ECART) / (taille.x + ECART)).floor() as usize).max(1);
                self.tv.colonnes = colonnes;
                let occupe = colonnes as f32 * taille.x + (colonnes - 1) as f32 * ECART;
                let marge = ((ui.available_width() - occupe) / 2.0).max(0.0);

                for ligne in ids.chunks(colonnes) {
                    ui.horizontal(|ui| {
                        ui.add_space(marge);
                        ui.spacing_mut().item_spacing.x = ECART;
                        for &id in ligne {
                            self.tv_tuile(ui, id, taille, cote, actions);
                        }
                    });
                    ui.add_space(ECART);
                }
                ui.add_space(12.0);
            });
        self.tv.defiler = false;
    }

    fn tv_tuile(&mut self, ui: &mut egui::Ui, id: u64, taille: Vec2, cote: f32, actions: &mut Vec<Action>) {
        let Some(jeu) = self.jeu(id).cloned() else { return };
        let p = theme::p();
        let selectionne = self.tv.selection == Some(id);
        let (rect, reponse) = ui.allocate_exact_size(taille, Sense::click());
        if selectionne && self.tv.defiler {
            ui.scroll_to_rect(rect, Some(egui::Align::Center));
        }
        if !ui.is_rect_visible(rect) {
            return;
        }
        let peintre = ui.painter_at(rect.expand(4.0));
        if selectionne {
            peintre.rect_filled(rect, CornerRadius::same(14), p.bg_btn);
            peintre.rect_stroke(
                rect,
                CornerRadius::same(14),
                Stroke::new(3.0, p.accent),
                egui::StrokeKind::Outside,
            );
        } else if reponse.hovered() {
            peintre.rect_filled(rect, CornerRadius::same(14), p.bg_tuile_hover);
        }

        let zone_icone = egui::Rect::from_min_size(rect.min + Vec2::new(14.0, 12.0), Vec2::splat(cote));
        let manquant = self.manquants.contains(&id);
        self.dessiner_icone(ui, &jeu, zone_icone, manquant);

        if jeu.favori {
            peintre.text(
                rect.right_top() + Vec2::new(-10.0, 8.0),
                Align2::RIGHT_TOP,
                "⭐",
                FontId::proportional(20.0),
                p.accent,
            );
        }

        let mut job = egui::text::LayoutJob::simple_singleline(
            jeu.nom.clone(),
            FontId::new(17.0, theme::gras()),
            if selectionne { p.fg } else { p.fg_muted },
        );
        job.halign = egui::Align::Center;
        job.wrap.max_width = rect.width() - 12.0;
        job.wrap.max_rows = 1;
        job.wrap.overflow_character = Some('…');
        let galley = ui.painter().layout_job(job);
        peintre.galley(
            egui::pos2(rect.center().x, zone_icone.bottom() + 10.0),
            galley,
            p.fg,
        );

        if reponse.clicked() {
            self.tv.selection = Some(id);
        }
        if reponse.double_clicked() {
            actions.push(Action::Lancer(id));
        }
    }

    fn tv_details(&mut self, ui: &mut egui::Ui) {
        let p = theme::p();
        let jeu = self.tv.selection.and_then(|id| self.jeu(id)).cloned();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                match &jeu {
                    Some(j) => {
                        ui.label(RichText::new(&j.nom).size(26.0).family(theme::gras()));
                        ui.label(RichText::new(resume_jeu(j)).size(16.0).color(p.fg_muted));
                        if !j.tags.is_empty() {
                            ui.label(RichText::new(format!("🏷 {}", j.tags.join("  ·  "))).color(p.fg_muted));
                        }
                        if self.manquants.contains(&j.id) {
                            ui.label(RichText::new("Exécutable introuvable").color(p.danger));
                        }
                    }
                    None => {
                        ui.label(RichText::new("Aucune sélection").size(22.0).color(p.fg_muted));
                    }
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Affichées de droite à gauche.
                let aides: [(&str, &str); 3] = if self.manette_connectee.is_some() {
                    [("B", "Quitter le plein écran"), ("Y", "Favori"), ("A", "Lancer")]
                } else {
                    [("Échap", "Quitter le plein écran"), ("F", "Favori"), ("Entrée", "Lancer")]
                };
                for (touche, texte) in aides {
                    ui.label(RichText::new(texte).color(p.fg));
                    badge(ui, touche);
                    ui.add_space(14.0);
                }
            });
        });
    }
}

/// Pastille de touche (« A », « LB », « Entrée »…).
fn badge(ui: &mut egui::Ui, texte: &str) {
    let p = theme::p();
    let galley = ui
        .painter()
        .layout_no_wrap(texte.to_string(), FontId::new(14.0, theme::gras()), p.fg);
    let (rect, _) = ui.allocate_exact_size(galley.size() + Vec2::new(16.0, 6.0), Sense::hover());
    ui.painter().rect(
        rect,
        CornerRadius::same(6),
        p.bg_btn_hover,
        Stroke::new(1.0, p.bordure),
        egui::StrokeKind::Inside,
    );
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, p.fg);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_dans_la_grille() {
        // 7 jeux sur 3 colonnes :  0 1 2 / 3 4 5 / 6
        assert_eq!(deplacer(0, 7, 3, Commande::Gauche), 0);
        assert_eq!(deplacer(0, 7, 3, Commande::Droite), 1);
        assert_eq!(deplacer(6, 7, 3, Commande::Droite), 6);
        assert_eq!(deplacer(1, 7, 3, Commande::Bas), 4);
        assert_eq!(deplacer(4, 7, 3, Commande::Bas), 6, "ligne incomplète");
        assert_eq!(deplacer(6, 7, 3, Commande::Bas), 6);
        assert_eq!(deplacer(4, 7, 3, Commande::Haut), 1);
        assert_eq!(deplacer(1, 7, 3, Commande::Haut), 1);
        assert_eq!(deplacer(0, 0, 3, Commande::Bas), 0);
    }
}
