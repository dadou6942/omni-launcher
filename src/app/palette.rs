//! Palette de commandes (Ctrl+K) et aide des raccourcis clavier.

use super::barre::{libelle_affichage, libelle_tri};
use super::*;

/// Lignes affichées au maximum, le reste s'obtient en précisant la recherche.
const RESULTATS_MAX: usize = 60;
const LARGEUR_PALETTE: f32 = 620.0;

pub(super) const RACCOURCIS: &[(&str, &str)] = &[
    ("Ctrl+K", "Palette de commandes"),
    ("Ctrl+F", "Rechercher un jeu"),
    ("Entrée", "Lancer le jeu sélectionné ou le premier résultat"),
    ("Échap", "Effacer la recherche ou fermer le panneau"),
    ("Ctrl+N", "Ajouter un jeu"),
    ("Ctrl+O", "Scanner un dossier"),
    ("Ctrl+B", "Replier la barre latérale"),
    ("F1", "Afficher cette aide"),
    ("F11", "Mode plein écran"),
];

#[derive(Clone)]
pub(super) struct EntreePalette {
    libelle: String,
    raccourci: &'static str,
    action: Action,
}

fn entree(libelle: impl Into<String>, raccourci: &'static str, action: Action) -> EntreePalette {
    EntreePalette {
        libelle: libelle.into(),
        raccourci,
        action,
    }
}

/// Minuscules sans accents, pour comparer ce que l'utilisateur tape.
fn normaliser(texte: &str) -> String {
    texte
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            autre => autre,
        })
        .collect()
}

/// `true` si tous les mots de la saisie figurent dans le libellé.
fn correspond(libelle: &str, saisie: &str) -> bool {
    let libelle = normaliser(libelle);
    saisie
        .split_whitespace()
        .all(|mot| libelle.contains(&normaliser(mot)))
}

#[derive(Default)]
pub(super) struct EtatPalette {
    saisie: String,
    index: usize,
}

impl App {
    fn entrees_generales(&self) -> Vec<EntreePalette> {
        let mut e = vec![
            entree("Ajouter un jeu", "Ctrl+N", Action::Ajouter),
            entree("Scanner un dossier", "Ctrl+O", Action::Scanner),
            entree("Re-scanner le dernier dossier", "", Action::Rescanner),
            entree("Aller à : Bibliothèque", "", Action::AllerSection(Section::Tout)),
            entree("Aller à : Récents", "", Action::AllerSection(Section::Recents)),
            entree("Aller à : Favoris", "", Action::AllerSection(Section::Favoris)),
            entree("Ouvrir les statistiques", "", Action::Ouvrir(Vue::Statistiques)),
            entree("Ouvrir les paramètres", "", Action::Ouvrir(Vue::Parametres)),
            entree("Mode plein écran", "F11", Action::BasculerPleinEcran),
            entree("Replier ou déplier la barre latérale", "Ctrl+B", Action::BasculerBarre),
            entree("Raccourcis clavier", "F1", Action::Aide),
            entree("Vérifier les jeux introuvables", "", Action::VerifierIntrouvables),
            entree("Retirer les jeux introuvables", "", Action::RetirerIntrouvables),
            entree("Récupérer les images manquantes", "", Action::RecupererImages),
            entree("Exporter la bibliothèque", "", Action::Exporter),
            entree("Importer une bibliothèque", "", Action::Importer),
            entree("Ouvrir le dossier des données", "", Action::OuvrirDonnees),
            entree("Vider le cache d'icônes", "", Action::ViderCache),
            entree("Vérifier les mises à jour", "", Action::VerifierMaj),
            entree("Quitter", "", Action::Quitter),
        ];
        for a in [Affichage::Grille, Affichage::Jaquettes, Affichage::Liste] {
            let libelle = format!("Vue : {}", libelle_affichage(a));
            e.push(entree(libelle, "", Action::ChangerAffichage(a)));
        }
        for t in [Tri::Nom, Tri::DernierLancement, Tri::DateAjout, Tri::TempsDeJeu] {
            e.push(entree(format!("Trier par : {}", libelle_tri(t)), "", Action::ChangerTri(t)));
        }
        for theme_choisi in theme::TOUS {
            let libelle = format!("Thème : {}", theme::nom(theme_choisi));
            e.push(entree(libelle, "", Action::ChangerTheme(theme_choisi)));
        }
        for (tag, _) in self.bib.config.tags() {
            let libelle = format!("Collection : {tag}");
            e.push(entree(libelle, "", Action::AllerSection(Section::Collection(tag))));
        }
        e
    }

    /// Actions du jeu sélectionné, proposées en premier.
    fn entrees_selection(&self) -> Vec<EntreePalette> {
        let Some(j) = self.selection.and_then(|id| self.bib.jeu(id)) else {
            return Vec::new();
        };
        let (id, nom) = (j.id, &j.nom);
        let favori = if j.favori { "Retirer des favoris" } else { "Ajouter aux favoris" };
        vec![
            entree(format!("{nom} : lancer"), "Entrée", Action::Lancer(id)),
            entree(format!("{nom} : {favori}"), "", Action::BasculerFavori(id)),
            entree(format!("{nom} : ouvrir l'emplacement"), "", Action::OuvrirEmplacement(id)),
            entree(format!("{nom} : tags"), "", Action::EditerTags(id)),
            entree(format!("{nom} : statut et notes"), "", Action::EditerNotes(id)),
            entree(format!("{nom} : renommer"), "", Action::DebuterRenommage(id)),
            entree(format!("{nom} : modifier l'icône"), "", Action::ChangerIcone(id)),
            entree(format!("{nom} : jaquette et bannière"), "", Action::EditerVisuels(id)),
            entree(format!("{nom} : arguments de lancement"), "", Action::EditerArguments(id)),
            entree(format!("{nom} : retirer du launcher"), "", Action::DemanderRetrait(id)),
            entree(format!("{nom} : supprimer du disque"), "", Action::DemanderSuppression(id)),
        ]
    }

    fn resultats_palette(&self, saisie: &str) -> Vec<EntreePalette> {
        let mut toutes = self.entrees_selection();
        toutes.extend(self.entrees_generales());
        if !saisie.trim().is_empty() {
            toutes.extend(
                self.bib
                    .config
                    .jeux
                    .iter()
                    .map(|j| entree(format!("Lancer {}", j.nom), "", Action::Lancer(j.id))),
            );
        }
        toutes
            .into_iter()
            .filter(|e| correspond(&e.libelle, saisie))
            .take(RESULTATS_MAX)
            .collect()
    }

    pub(super) fn fenetre_palette(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let Some(etat) = &self.palette else { return };
        let resultats = self.resultats_palette(&etat.saisie);
        let p = theme::p();
        let n = resultats.len();

        // Avant le champ de saisie, qui garderait ces touches pour lui.
        let (bas, haut, entree_key, echap) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, Key::ArrowDown),
                i.consume_key(egui::Modifiers::NONE, Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, Key::Enter),
                i.consume_key(egui::Modifiers::NONE, Key::Escape),
            )
        });
        let Some(etat) = &mut self.palette else { return };
        if bas && n > 0 {
            etat.index = (etat.index + 1) % n;
        }
        if haut && n > 0 {
            etat.index = (etat.index + n - 1) % n;
        }
        etat.index = etat.index.min(n.saturating_sub(1));
        let mut choisi = entree_key.then(|| resultats.get(etat.index).cloned()).flatten();

        let id = egui::Id::unique("palette_commandes");
        let zone = egui::Modal::default_area(id).anchor(Align2::CENTER_TOP, [0.0, 90.0]);
        let reponse = egui::Modal::new(id).area(zone).show(ctx, |ui| {
            ui.set_width(LARGEUR_PALETTE.min(ctx.content_rect().width() - theme::ESPACE_XXL));
            let champ = ui.add(
                egui::TextEdit::singleline(&mut etat.saisie)
                    .hint_text("Tapez une commande ou le nom d'un jeu…")
                    .desired_width(f32::INFINITY)
                    .margin(egui::vec2(theme::ESPACE_M, theme::ESPACE_S)),
            );
            champ.request_focus();
            if champ.changed() {
                etat.index = 0;
            }
            ui.add_space(theme::ESPACE_S);
            egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                if resultats.is_empty() {
                    ui.label(RichText::new("Aucune commande trouvée.").color(p.fg_muted));
                }
                for (i, e) in resultats.iter().enumerate() {
                    let actif = i == etat.index;
                    let (rect, r) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 34.0),
                        Sense::click(),
                    );
                    if actif {
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same(theme::RAYON_S),
                            p.accent.gamma_multiply(0.16),
                        );
                        if bas || haut {
                            ui.scroll_to_rect(rect, None);
                        }
                    }
                    ui.painter().text(
                        rect.left_center() + Vec2::new(theme::ESPACE_M, 0.0),
                        Align2::LEFT_CENTER,
                        &e.libelle,
                        FontId::proportional(theme::POLICE_CORPS),
                        if actif { p.accent } else { p.fg },
                    );
                    ui.painter().text(
                        rect.right_center() - Vec2::new(theme::ESPACE_M, 0.0),
                        Align2::RIGHT_CENTER,
                        e.raccourci,
                        FontId::proportional(theme::POLICE_LEGENDE),
                        p.fg_muted,
                    );
                    if r.clicked() {
                        choisi = Some(e.clone());
                    }
                }
            });
        });

        if let Some(e) = choisi {
            self.palette = None;
            actions.push(e.action);
        } else if echap || reponse.should_close() {
            self.palette = None;
        }
    }

    pub(super) fn fenetre_aide(&mut self, ctx: &egui::Context) {
        if !self.aide_ouverte {
            return;
        }
        let reponse = egui::Modal::new(egui::Id::unique("aide_raccourcis")).show(ctx, |ui| {
            ui.set_width(460.0);
            ui.heading("Raccourcis clavier");
            ui.add_space(theme::ESPACE_S);
            tableau_raccourcis(ui);
            ui.add_space(theme::ESPACE_M);
            ui.button("Fermer").clicked()
        });
        if reponse.inner || reponse.should_close() {
            self.aide_ouverte = false;
        }
    }
}

pub(super) fn tableau_raccourcis(ui: &mut egui::Ui) {
    let p = theme::p();
    egui::Grid::new("raccourcis")
        .num_columns(2)
        .spacing([theme::ESPACE_L, theme::ESPACE_S])
        .show(ui, |ui| {
            for (touche, effet) in RACCOURCIS {
                ui.label(RichText::new(*touche).strong().color(p.accent));
                ui.label(*effet);
                ui.end_row();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tous_les_mots_doivent_correspondre() {
        assert!(correspond("Vérifier les mises à jour", "mises jour"));
        assert!(correspond("Vérifier les mises à jour", "VÉRIFIER"));
        assert!(correspond("Thème : Clair", "theme"));
        assert!(!correspond("Vérifier les mises à jour", "mises tags"));
        assert!(correspond("Quitter", ""));
    }
}
