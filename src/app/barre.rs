//! Barre latérale repliable et barre du haut du mode bureau.

use super::*;

const LARGEUR_BARRE: f32 = 224.0;
const LARGEUR_REPLIEE: f32 = 60.0;
const HAUTEUR_ENTREE: f32 = 38.0;

/// Ligne cliquable de la barre latérale.
fn entree(
    ui: &mut egui::Ui,
    icone: &str,
    libelle: &str,
    compte: Option<usize>,
    actif: bool,
    replie: bool,
) -> egui::Response {
    let p = theme::p();
    let largeur = ui.available_width();
    let (rect, reponse) =
        ui.allocate_exact_size(Vec2::new(largeur, HAUTEUR_ENTREE), Sense::click());
    let arrondi = CornerRadius::same(theme::RAYON_M);
    let peintre = ui.painter_at(rect);
    if actif {
        peintre.rect_filled(rect, arrondi, p.accent.gamma_multiply(0.14));
    } else if reponse.hovered() {
        peintre.rect_filled(rect, arrondi, p.bg_btn);
    }
    let couleur = if actif { p.accent } else { p.fg };
    let x_icone = rect.left() + (LARGEUR_REPLIEE - theme::ESPACE_L) / 2.0 - theme::ESPACE_S;
    peintre.text(
        egui::pos2(x_icone, rect.center().y),
        Align2::CENTER_CENTER,
        icone,
        FontId::proportional(theme::POLICE_SECTION),
        couleur,
    );
    if !replie {
        peintre.text(
            egui::pos2(x_icone + theme::ESPACE_L + theme::ESPACE_S, rect.center().y),
            Align2::LEFT_CENTER,
            libelle,
            FontId::proportional(theme::POLICE_CORPS),
            couleur,
        );
        if let Some(n) = compte {
            peintre.text(
                egui::pos2(rect.right() - theme::ESPACE_M, rect.center().y),
                Align2::RIGHT_CENTER,
                n.to_string(),
                FontId::proportional(theme::POLICE_LEGENDE),
                p.fg_muted,
            );
        }
    }
    let reponse = reponse.on_hover_cursor(egui::CursorIcon::PointingHand);
    if replie {
        reponse.on_hover_text(libelle)
    } else {
        reponse
    }
}

impl App {
    fn section_active(&self, section: &Section) -> bool {
        self.vue == Vue::Bibliotheque && self.section == *section
    }

    pub(super) fn barre_laterale(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let p = theme::p();
        let cible = if self.bib.config.barre_repliee {
            LARGEUR_REPLIEE
        } else {
            LARGEUR_BARRE
        };
        let largeur = ui.ctx().animate_value_with_time(
            egui::Id::unique("largeur_barre"),
            cible,
            theme::DUREE_NORMALE,
        );
        let replie = largeur < (LARGEUR_BARRE + LARGEUR_REPLIEE) / 2.0;
        let cadre = egui::Frame::new()
            .fill(p.bg_menu)
            .inner_margin(egui::Margin::same(theme::ESPACE_S as i8));

        egui::Panel::left("barre_laterale")
            .exact_size(largeur)
            .resizable(false)
            .show_separator_line(false)
            .frame(cadre)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = theme::ESPACE_XS;
                ui.add_space(theme::ESPACE_S);
                if entree(ui, "☰", "Omni Launcher", None, false, replie)
                    .on_hover_text("Replier ou déplier la barre (Ctrl+B)")
                    .clicked()
                {
                    actions.push(Action::BasculerBarre);
                }
                ui.add_space(theme::ESPACE_M);

                let nb_jeux = self.bib.config.jeux.len();
                let favoris = self.bib.config.jeux.iter().filter(|j| j.favori).count();
                let recents = self
                    .bib
                    .config
                    .jeux
                    .iter()
                    .filter(|j| j.dernier_lancement.is_some())
                    .count();
                let sections = [
                    ("🏠", "Bibliothèque", Section::Tout, nb_jeux),
                    ("🕑", "Récents", Section::Recents, recents),
                    ("⭐", "Favoris", Section::Favoris, favoris),
                ];
                for (icone, libelle, section, n) in sections {
                    let actif = self.section_active(&section);
                    if entree(ui, icone, libelle, Some(n), actif, replie).clicked() {
                        actions.push(Action::AllerSection(section));
                    }
                }

                let tags = self.bib.config.tags();
                let dans_collection = matches!(self.section, Section::Collection(_));
                let actif = self.vue == Vue::Bibliotheque && dans_collection;
                if entree(ui, "🏷", "Collections", Some(tags.len()), actif, replie).clicked() {
                    match tags.first() {
                        Some(_) if replie => actions.push(Action::BasculerBarre),
                        Some((t, _)) => {
                            actions.push(Action::AllerSection(Section::Collection(t.clone())));
                        }
                        None => self.notifier(
                            "Aucune collection. Clic droit sur un jeu ➔ Tags… pour en créer une.",
                        ),
                    }
                }

                let bas = 3.0 * (HAUTEUR_ENTREE + theme::ESPACE_XS) + theme::ESPACE_M;
                if !replie {
                    let hauteur = (ui.available_height() - bas).max(0.0);
                    egui::ScrollArea::vertical()
                        .max_height(hauteur)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for (tag, n) in tags {
                                let section = Section::Collection(tag.clone());
                                let actif = self.section_active(&section);
                                if entree(ui, " ", &tag, Some(n), actif, replie).clicked() {
                                    actions.push(Action::AllerSection(section));
                                }
                            }
                        });
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.spacing_mut().item_spacing.y = theme::ESPACE_XS;
                    ui.add_space(theme::ESPACE_XS);
                    if entree(ui, "❓", "Raccourcis clavier", None, false, replie).clicked() {
                        actions.push(Action::Aide);
                    }
                    let actif = self.vue == Vue::Parametres;
                    if entree(ui, "⚙", "Paramètres", None, actif, replie).clicked() {
                        actions.push(Action::Ouvrir(Vue::Parametres));
                    }
                    let actif = self.vue == Vue::Statistiques;
                    if entree(ui, "📊", "Statistiques", None, actif, replie).clicked() {
                        actions.push(Action::Ouvrir(Vue::Statistiques));
                    }
                });
            });
    }

    pub(super) fn barre_haut(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let p = theme::p();
        ui.add_space(theme::ESPACE_M);
        ui.horizontal(|ui| {
            ui.add_space(theme::ESPACE_XL);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(theme::ESPACE_XL);
                if ui
                    .button("🎮")
                    .on_hover_text("Mode plein écran à la manette (F11)")
                    .clicked()
                {
                    actions.push(Action::BasculerPleinEcran);
                }
                let scan = egui::Button::new("🗁  Scanner un dossier");
                if ui
                    .add_enabled(self.scan.is_none(), scan)
                    .on_hover_text("Ajoute tous les jeux d'un dossier, avec un aperçu avant import")
                    .clicked()
                {
                    actions.push(Action::Scanner);
                }
                let ajouter = egui::Button::new(RichText::new("+  Ajouter").color(p.sur_accent()))
                    .fill(p.accent);
                if ui
                    .add(ajouter)
                    .on_hover_text("Ajouter un ou plusieurs .exe (Ctrl+N)")
                    .clicked()
                {
                    actions.push(Action::Ajouter);
                }
                self.bouton_maj(ui);
                ui.add_space(theme::ESPACE_S);
                self.selecteur_affichage(ui);
                self.selecteur_tri(ui);
                if ui
                    .button(RichText::new("Ctrl+K").color(p.fg_muted))
                    .on_hover_text("Palette de commandes : toutes les actions")
                    .clicked()
                {
                    actions.push(Action::Palette);
                }

                let rech = ui.add(
                    egui::TextEdit::singleline(&mut self.recherche)
                        .hint_text("🔍  Rechercher (Ctrl+F)")
                        .desired_width((ui.available_width() - theme::ESPACE_S).max(120.0))
                        .margin(egui::vec2(theme::ESPACE_M, theme::ESPACE_S)),
                );
                if rech.changed() {
                    self.vue = Vue::Bibliotheque;
                }
                if self.focus_recherche {
                    rech.request_focus();
                    self.focus_recherche = false;
                }
                if rech.lost_focus() {
                    let (entree, echap) =
                        ui.input(|i| (i.key_pressed(Key::Enter), i.key_pressed(Key::Escape)));
                    if echap {
                        self.recherche.clear();
                    } else if entree && !self.recherche.is_empty() {
                        // Entrée : lance le premier résultat.
                        if let Some(&id) = self.ids_visibles().first() {
                            actions.push(Action::Lancer(id));
                        }
                    }
                }
            });
        });
        ui.add_space(theme::ESPACE_M);
    }

    fn selecteur_affichage(&mut self, ui: &mut egui::Ui) {
        let avant = self.bib.config.affichage;
        egui::ComboBox::from_id_salt("affichage")
            .selected_text(libelle_affichage(avant))
            .show_ui(ui, |ui| {
                for a in [Affichage::Grille, Affichage::Jaquettes, Affichage::Liste] {
                    ui.selectable_value(&mut self.bib.config.affichage, a, libelle_affichage(a));
                }
            });
        if avant != self.bib.config.affichage {
            self.sauver();
        }
    }

    fn selecteur_tri(&mut self, ui: &mut egui::Ui) {
        let avant = self.bib.config.tri;
        egui::ComboBox::from_id_salt("tri")
            .selected_text(format!("Tri : {}", libelle_tri(avant).to_lowercase()))
            .show_ui(ui, |ui| {
                for t in [Tri::Nom, Tri::DernierLancement, Tri::DateAjout, Tri::TempsDeJeu] {
                    ui.selectable_value(&mut self.bib.config.tri, t, libelle_tri(t));
                }
            });
        if avant != self.bib.config.tri {
            self.sauver();
        }
    }
}

pub(super) fn libelle_affichage(a: Affichage) -> &'static str {
    match a {
        Affichage::Grille => "Icônes",
        Affichage::Jaquettes => "Jaquettes",
        Affichage::Liste => "Liste",
    }
}

pub(super) fn libelle_tri(t: Tri) -> &'static str {
    match t {
        Tri::Nom => "Nom",
        Tri::DernierLancement => "Récents",
        Tri::DateAjout => "Date d'ajout",
        Tri::TempsDeJeu => "Temps de jeu",
    }
}
