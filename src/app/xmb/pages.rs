//! Contenu des pages du menu XMB : chaque page est une liste d'éléments, et
//! chaque élément déclenche un effet. Tout passe par la même navigation.

use super::fichiers::{self, EntreeFichier};
use super::*;
use crate::config::{Glyphes, StylePleinEcran, Theme};
use crate::maj;

const ACCENTS: [(&str, [u8; 3]); 6] = [
    ("Vert tilleul", [0xde, 0xff, 0x9a]),
    ("Bleu", [0x5a, 0xa9, 0xff]),
    ("Violet", [0xb9, 0x8c, 0xff]),
    ("Rose", [0xff, 0x7e, 0xb6]),
    ("Orange", [0xff, 0xa8, 0x5a]),
    ("Rouge", [0xff, 0x6b, 0x6b]),
];
const TAILLES_TUILE: [f32; 5] = [90.0, 120.0, 140.0, 180.0, 220.0];

/// Où mène la sélection d'un fichier ou d'un dossier.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum But {
    Ajouter,
    Scanner,
    Localiser(u64),
    Icone(u64),
    Image(u64, Genre),
    Exporter,
    Importer,
}

impl But {
    fn extensions(&self) -> &'static [&'static str] {
        match self {
            But::Ajouter | But::Localiser(_) => &["exe"],
            But::Icone(_) => &["png", "jpg", "jpeg", "ico", "bmp", "webp"],
            But::Image(..) => &["png", "jpg", "jpeg", "webp"],
            But::Importer => &["json"],
            But::Scanner | But::Exporter => &[],
        }
    }

    fn cherche_un_dossier(&self) -> bool {
        matches!(self, But::Scanner | But::Exporter)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Page {
    Collection(String),
    ParStatut(Statut),
    Recherche,
    OptionsJeu(u64),
    StatutJeu(u64),
    TagsJeu(u64),
    ImagesJeu(u64),
    Tri,
    Bibliotheque,
    Apparence,
    Themes,
    Accents,
    TaillesTuile,
    PleinEcran,
    StyleEcran,
    ChoixGlyphes,
    Images,
    Tags,
    OptionsTag(String),
    Sauvegardes,
    Donnees,
    MisesAJour,
    Statistiques,
    Astuces,
    Fichiers(But, Option<PathBuf>),
    Revue,
    Dialogue,
}

/// Texte à saisir au clavier virtuel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Saisie {
    Recherche,
    Renommer(u64),
    Notes(u64),
    Arguments(u64),
    NouveauTag(u64),
    RenommerTag(String),
    CleApi,
    RechercheImages(u64),
    NomCandidat(usize),
    Accent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Reglage {
    Recents,
    FermerFin,
    FondAnime,
    Sons,
    DemarrerPleinEcran,
    BasculeManette,
    VerifierMaj,
}

/// Effets propres au menu, sans équivalent dans [`Action`].
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Interne {
    Reglage(Reglage),
    Accent(Option<[u8; 3]>),
    Style(StylePleinEcran),
    Glyphes(Glyphes),
    Taille(f32),
    Statut(u64, Statut),
    BasculerTag(u64, String),
    SupprimerTag(String),
    ConfirmerSuppressionTag(String),
    RetirerImage(u64, Genre),
    Telecharger(u64),
    OuvrirSauvegardes,
    OuvrirPageMaj(String),
    Restaurer(usize),
    Selection(But, PathBuf),
    Entrer(But, PathBuf),
    ToutCocher(bool),
    Principaux,
    BasculerCandidat(usize),
    ImporterRevue,
    AnnulerRevue,
    ConfirmerRetrait(u64),
    ConfirmerSuppression(u64, PathBuf),
    ConfirmerIntrouvables,
    ConfirmerRestauration(PathBuf),
    ImportFusion,
    ImportRemplacer,
    FermerDialogue,
    ModeBureau,
}

#[derive(Clone)]
pub(super) enum Effet {
    Aucun,
    Action(Action),
    Ouvrir(Page),
    Saisir(String, String, Saisie),
    Interne(Interne),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Visuel {
    Jeu(u64),
    Glyphe(&'static str),
}

#[derive(Clone)]
pub(super) struct Element {
    pub(super) titre: String,
    pub(super) detail: String,
    pub(super) visuel: Visuel,
    pub(super) effet: Effet,
}

fn el(glyphe: &'static str, titre: impl Into<String>, detail: impl Into<String>, effet: Effet) -> Element {
    Element {
        titre: titre.into(),
        detail: detail.into(),
        visuel: Visuel::Glyphe(glyphe),
        effet,
    }
}

fn info(titre: impl Into<String>, detail: impl Into<String>) -> Element {
    el("•", titre, detail, Effet::Aucun)
}

fn page(glyphe: &'static str, titre: &str, detail: &str, p: Page) -> Element {
    el(glyphe, titre, detail, Effet::Ouvrir(p))
}

fn action(glyphe: &'static str, titre: impl Into<String>, detail: impl Into<String>, a: Action) -> Element {
    el(glyphe, titre, detail, Effet::Action(a))
}

fn interne(glyphe: &'static str, titre: impl Into<String>, detail: impl Into<String>, i: Interne) -> Element {
    el(glyphe, titre, detail, Effet::Interne(i))
}

fn saisir(glyphe: &'static str, titre: &str, detail: impl Into<String>, initial: String, s: Saisie) -> Element {
    el(glyphe, titre, detail, Effet::Saisir(titre.to_string(), initial, s))
}

fn coche(actif: bool) -> &'static str {
    if actif { "●" } else { "○" }
}

fn etat(actif: bool) -> &'static str {
    if actif { "Activé" } else { "Désactivé" }
}

impl App {
    fn elements_jeux(&self, section: &Section, recherche: &str, statut: Option<Statut>) -> Vec<Element> {
        self.bib
            .lister_jeux(Filtre { section, recherche, statut })
            .into_iter()
            .map(|j| Element {
                titre: j.nom.clone(),
                // Calculé au dessin, pour les seuls éléments visibles.
                detail: String::new(),
                visuel: Visuel::Jeu(j.id),
                effet: Effet::Action(Action::Lancer(j.id)),
            })
            .collect()
    }

    fn sans_jeu(texte: &str) -> Vec<Element> {
        vec![
            info(texte, ""),
            page("+", "Ajouter un jeu", "Choisir un exécutable", Page::Fichiers(But::Ajouter, None)),
            page("🗁", "Scanner un dossier", "Importer tous les jeux d'un dossier", Page::Fichiers(But::Scanner, None)),
        ]
    }

    /// Éléments d'une catégorie de la barre horizontale.
    pub(super) fn elements_categorie(&self, categorie: Categorie) -> Vec<Element> {
        let liste = |section: &Section, vide: &str| {
            let e = self.elements_jeux(section, "", None);
            if e.is_empty() { Self::sans_jeu(vide) } else { e }
        };
        match categorie {
            Categorie::Jeux => liste(&Section::Tout, "Aucun jeu dans la bibliothèque"),
            Categorie::Recents => liste(&Section::Recents, "Aucun jeu lancé pour l'instant"),
            Categorie::Favoris => liste(&Section::Favoris, "Aucun favori pour l'instant"),
            Categorie::Collections => self.elements_collections(),
            Categorie::Parametres => vec![
                page("🏠", "Bibliothèque", "Ajouter, scanner, trier, vérifier", Page::Bibliotheque),
                page("🖼", "Apparence", "Thème et couleur d'accent", Page::Apparence),
                page("🎮", "Plein écran et manette", "Style, boutons, fond, sons, démarrage", Page::PleinEcran),
                page("🖼", "Jaquettes et bannières", "Clé SteamGridDB, images manquantes", Page::Images),
                page("🏷", "Tags", "Renommer ou supprimer", Page::Tags),
                el("🗁", "Sauvegardes", "Restaurer, exporter, importer", Effet::Interne(Interne::OuvrirSauvegardes)),
                page("⬆", "Mises à jour", format!("Version {}", maj::VERSION_ACTUELLE).as_str(), Page::MisesAJour),
                page("🗁", "Données", "Dossier des données, cache d'icônes", Page::Donnees),
                page("📊", "Statistiques", "Temps de jeu et classements", Page::Statistiques),
                page("❓", "Astuces", "", Page::Astuces),
            ],
            Categorie::Quitter => vec![
                interne("🏠", "Revenir au mode bureau", "Quitte le plein écran", Interne::ModeBureau),
                action("×", "Quitter Omni Launcher", "", Action::Quitter),
            ],
        }
    }

    fn elements_collections(&self) -> Vec<Element> {
        let mut e: Vec<Element> = self
            .bib
            .config
            .tags()
            .into_iter()
            .map(|(tag, n)| el("🏷", tag.clone(), format!("{n} jeu(x)"), Effet::Ouvrir(Page::Collection(tag))))
            .collect();
        e.extend(self.bib.config.statuts().into_iter().map(|(s, n)| {
            el("•", s.libelle(), format!("{n} jeu(x)"), Effet::Ouvrir(Page::ParStatut(s)))
        }));
        if e.is_empty() {
            e.push(info("Aucune collection", "Bouton d'options sur un jeu, puis Tags"));
        }
        e
    }

    /// Éléments d'une sous-page.
    pub(super) fn elements_page(&mut self, page_courante: &Page) -> Vec<Element> {
        match page_courante {
            Page::Collection(t) => self.elements_jeux(&Section::Collection(t.clone()), "", None),
            Page::ParStatut(s) => self.elements_jeux(&Section::Tout, "", Some(*s)),
            Page::Recherche => {
                let e = self.elements_jeux(&Section::Tout, &self.recherche, None);
                if e.is_empty() {
                    vec![info(format!("Aucun résultat pour « {} »", self.recherche), "")]
                } else {
                    e
                }
            }
            Page::OptionsJeu(id) => self.elements_options_jeu(*id),
            Page::StatutJeu(id) => self.elements_statut(*id),
            Page::TagsJeu(id) => self.elements_tags_jeu(*id),
            Page::ImagesJeu(id) => self.elements_images_jeu(*id),
            Page::Tri => [Tri::Nom, Tri::DernierLancement, Tri::DateAjout, Tri::TempsDeJeu]
                .into_iter()
                .map(|t| {
                    let g = coche(self.bib.config.tri == t);
                    action(g, barre::libelle_tri(t), "", Action::ChangerTri(t))
                })
                .collect(),
            Page::Bibliotheque => self.elements_bibliotheque(),
            Page::Apparence => vec![
                page("🖼", "Thème", theme::nom(self.bib.config.theme), Page::Themes),
                page("●", "Couleur d'accent", "", Page::Accents),
            ],
            Page::Themes => theme::TOUS
                .into_iter()
                .map(|t: Theme| {
                    action(coche(self.bib.config.theme == t), theme::nom(t), "", Action::ChangerTheme(t))
                })
                .collect(),
            Page::Accents => self.elements_accents(),
            Page::TaillesTuile => TAILLES_TUILE
                .into_iter()
                .map(|t| {
                    let g = coche((self.bib.config.taille_tuile - t).abs() < 1.0);
                    interne(g, format!("{t:.0} px"), "", Interne::Taille(t))
                })
                .collect(),
            Page::PleinEcran => self.elements_plein_ecran(),
            Page::StyleEcran => [
                (StylePleinEcran::Xmb, "Menu XMB", "Catégories et liste verticale"),
                (StylePleinEcran::Grille, "Grille", "Grandes tuiles"),
            ]
            .into_iter()
            .map(|(s, t, d)| interne(coche(self.bib.config.style_plein_ecran == s), t, d, Interne::Style(s)))
            .collect(),
            Page::ChoixGlyphes => [
                (Glyphes::Auto, "Automatique", "Selon la manette branchée"),
                (Glyphes::Xbox, "Xbox", "A, B, X, Y"),
                (Glyphes::PlayStation, "PlayStation", "Croix, rond, carré, triangle"),
            ]
            .into_iter()
            .map(|(g, t, d)| interne(coche(self.bib.config.glyphes == g), t, d, Interne::Glyphes(g)))
            .collect(),
            Page::Images => self.elements_images(),
            Page::Tags => self.elements_tags(),
            Page::OptionsTag(t) => vec![
                saisir("✏", "Renommer le tag", "", t.clone(), Saisie::RenommerTag(t.clone())),
                interne("🗑", "Supprimer le tag", "Les jeux ne sont pas touchés", Interne::SupprimerTag(t.clone())),
            ],
            Page::Sauvegardes => self.elements_sauvegardes(),
            Page::Donnees => vec![
                info("Emplacement", self.bib.dossiers.racine.display().to_string()),
                action("🗁", "Ouvrir le dossier des données", "", Action::OuvrirDonnees),
                action("🗑", "Vider le cache d'icônes", "", Action::ViderCache),
            ],
            Page::MisesAJour => self.elements_maj(),
            Page::Statistiques => self.elements_stats(),
            Page::Astuces => vec![
                info("Options d'un jeu", "Bouton d'options sur un jeu"),
                info("Recherche", "Bouton de recherche, avec le clavier virtuel"),
                info("Catégories", "Gâchettes hautes, ou gauche et droite"),
                info("Suppression", "Le dossier du jeu part à la corbeille : c'est récupérable"),
            ],
            Page::Fichiers(but, dossier) => self.elements_fichiers(but, dossier.as_deref()),
            Page::Revue => self.elements_revue(),
            Page::Dialogue => self.elements_dialogue(),
        }
    }

    fn elements_options_jeu(&self, id: u64) -> Vec<Element> {
        let Some(j) = self.bib.jeu(id) else {
            return vec![info("Jeu introuvable", "")];
        };
        let mut e = vec![
            action("▶", "Lancer", "", Action::Lancer(id)),
            action(
                "⭐",
                if j.favori { "Retirer des favoris" } else { "Ajouter aux favoris" },
                "",
                Action::BasculerFavori(id),
            ),
            page("•", "Statut", j.statut.libelle(), Page::StatutJeu(id)),
            saisir("📝", "Notes", apercu_notes(&j.notes), j.notes.clone(), Saisie::Notes(id)),
            page("🏷", "Tags", &j.tags.join(", "), Page::TagsJeu(id)),
            saisir("✏", "Renommer", "", j.nom.clone(), Saisie::Renommer(id)),
            page("🖼", "Jaquette et bannière", "", Page::ImagesJeu(id)),
            page("🖼", "Modifier l'icône", "", Page::Fichiers(But::Icone(id), None)),
        ];
        if j.icone_perso.is_some() {
            e.push(action("↺", "Rétablir l'icône d'origine", "", Action::RetablirIcone(id)));
        }
        e.extend([
            saisir("⚙", "Arguments de lancement", j.arguments.clone(), j.arguments.clone(), Saisie::Arguments(id)),
            action("🗁", "Ouvrir l'emplacement", j.exe.display().to_string(), Action::OuvrirEmplacement(id)),
            page("☰", "Trier la bibliothèque", barre::libelle_tri(self.bib.config.tri), Page::Tri),
            action("➖", "Retirer du launcher", "Le jeu reste installé", Action::DemanderRetrait(id)),
            action("🗑", "Supprimer le jeu du disque", "Envoi à la corbeille", Action::DemanderSuppression(id)),
        ]);
        e
    }

    fn elements_statut(&self, id: u64) -> Vec<Element> {
        let actuel = self.bib.jeu(id).map(|j| j.statut);
        Statut::TOUS
            .into_iter()
            .map(|s| interne(coche(actuel == Some(s)), s.libelle(), "", Interne::Statut(id, s)))
            .collect()
    }

    fn elements_tags_jeu(&self, id: u64) -> Vec<Element> {
        let Some(j) = self.bib.jeu(id) else {
            return Vec::new();
        };
        let mut e = vec![saisir("+", "Nouveau tag", "", String::new(), Saisie::NouveauTag(id))];
        e.extend(self.bib.config.tags().into_iter().map(|(t, n)| {
            let g = coche(j.a_tag(&t));
            interne(g, t.clone(), format!("{n} jeu(x)"), Interne::BasculerTag(id, t))
        }));
        e
    }

    fn elements_images_jeu(&self, id: u64) -> Vec<Element> {
        let Some(j) = self.bib.jeu(id) else {
            return Vec::new();
        };
        let cle = !self.cle_sgdb.trim().is_empty();
        let mut e = Vec::new();
        if cle {
            e.push(saisir("🔍", "Chercher sur SteamGridDB", "", j.nom.clone(), Saisie::RechercheImages(id)));
        } else {
            e.push(info("SteamGridDB", "Ajoutez votre clé dans Paramètres, Jaquettes et bannières"));
        }
        if let Some(edition) = &self.edition_visuels {
            if edition.occupe() {
                e.push(info("Recherche en cours…", ""));
            }
            if let Some(m) = edition.message() {
                e.push(info(m.to_string(), ""));
            }
            e.extend(
                edition
                    .resultats()
                    .iter()
                    .map(|r| interne("⬇", r.nom.clone(), "Télécharger les images", Interne::Telecharger(r.id))),
            );
        }
        for (genre, libelle, actuel) in [
            (Genre::Jaquette, "Jaquette", &j.jaquette),
            (Genre::Banniere, "Bannière", &j.banniere),
        ] {
            e.push(page(
                "🖼",
                &format!("{libelle} : choisir une image"),
                if actuel.is_some() { "Définie" } else { "Aucune" },
                Page::Fichiers(But::Image(id, genre), None),
            ));
            if actuel.is_some() {
                e.push(interne("🗑", format!("{libelle} : retirer"), "", Interne::RetirerImage(id, genre)));
            }
        }
        e
    }

    fn elements_bibliotheque(&self) -> Vec<Element> {
        let c = &self.bib.config;
        let dernier = c
            .dernier_dossier_scan
            .as_ref()
            .map(|d| d.display().to_string())
            .unwrap_or_else(|| "Aucun dossier scanné".to_string());
        let favoris = c.jeux.iter().filter(|j| j.favori).count();
        vec![
            info(format!("{} jeux", c.jeux.len()), format!("Dont {favoris} favoris")),
            page("+", "Ajouter un jeu", "Choisir un exécutable", Page::Fichiers(But::Ajouter, None)),
            page("🗁", "Scanner un dossier", "", Page::Fichiers(But::Scanner, None)),
            action("🔄", "Re-scanner le dernier dossier", dernier, Action::Rescanner),
            page("☰", "Trier par", barre::libelle_tri(c.tri), Page::Tri),
            page("☰", "Taille des tuiles", format!("{:.0} px", c.taille_tuile).as_str(), Page::TaillesTuile),
            interne(coche(c.afficher_recents), "Section « Joués récemment »", etat(c.afficher_recents), Interne::Reglage(Reglage::Recents)),
            interne(coche(c.fermer_apres_lancement), "Fermer le launcher à la fin de la partie", "En mode bureau uniquement", Interne::Reglage(Reglage::FermerFin)),
            action("✔", "Vérifier les jeux introuvables", "", Action::VerifierIntrouvables),
            action("🗑", "Retirer les jeux introuvables", "", Action::RetirerIntrouvables),
        ]
    }

    fn elements_accents(&self) -> Vec<Element> {
        let actuel = self.bib.config.accent_perso;
        let mut e = vec![interne(coche(actuel.is_none()), "Couleur du thème", "", Interne::Accent(None))];
        e.extend(ACCENTS.into_iter().map(|(nom, c)| {
            interne(coche(actuel == Some(c)), nom, "", Interne::Accent(Some(c)))
        }));
        e.push(saisir("✏", "Couleur libre", "Code hexadécimal, par exemple ff8800", String::new(), Saisie::Accent));
        e
    }

    fn elements_plein_ecran(&self) -> Vec<Element> {
        let c = &self.bib.config;
        let style = match c.style_plein_ecran {
            StylePleinEcran::Xmb => "Menu XMB",
            StylePleinEcran::Grille => "Grille",
        };
        let glyphes = match c.glyphes {
            Glyphes::Auto => "Automatique",
            Glyphes::Xbox => "Xbox",
            Glyphes::PlayStation => "PlayStation",
        };
        let mut reglages = vec![
            page("☰", "Style du plein écran", style, Page::StyleEcran),
            page("🎮", "Boutons affichés", glyphes, Page::ChoixGlyphes),
        ];
        for (r, titre, actif) in [
            (Reglage::FondAnime, "Fond animé", c.fond_anime),
            (Reglage::Sons, "Sons de navigation", c.sons),
            (Reglage::DemarrerPleinEcran, "Démarrer en plein écran", c.demarrer_plein_ecran),
            (Reglage::BasculeManette, "Plein écran quand une manette est branchée", c.bascule_manette),
        ] {
            reglages.push(interne(coche(actif), titre, etat(actif), Interne::Reglage(r)));
        }
        reglages
    }

    fn elements_images(&self) -> Vec<Element> {
        let cle = self.cle_sgdb.trim();
        let etat_cle = if cle.is_empty() { "Aucune clé" } else { "Clé enregistrée" };
        vec![
            saisir("✏", "Clé d'API SteamGridDB", etat_cle, self.cle_sgdb.clone(), Saisie::CleApi),
            action("🗁", "Obtenir une clé", "Ouvre SteamGridDB dans le navigateur", Action::OuvrirPageCle),
            action("⬇", "Récupérer les images manquantes", "Prend le premier résultat de chaque jeu", Action::RecupererImages),
        ]
    }

    fn elements_tags(&self) -> Vec<Element> {
        let tags = self.bib.config.tags();
        if tags.is_empty() {
            return vec![info("Aucun tag", "Bouton d'options sur un jeu, puis Tags")];
        }
        tags.into_iter()
            .map(|(t, n)| el("🏷", t.clone(), format!("{n} jeu(x)"), Effet::Ouvrir(Page::OptionsTag(t))))
            .collect()
    }

    fn elements_sauvegardes(&self) -> Vec<Element> {
        let mut e = vec![
            page("⬆", "Exporter la bibliothèque", "Choisir un dossier", Page::Fichiers(But::Exporter, None)),
            page("⬇", "Importer une bibliothèque", "Choisir un fichier .json", Page::Fichiers(But::Importer, None)),
            action("🗁", "Ouvrir le dossier des sauvegardes", "", Action::OuvrirSauvegardes),
        ];
        e.extend(self.sauvegardes.iter().enumerate().map(|(i, s)| {
            let detail = match s.nb_jeux {
                Some(n) => format!("{n} jeu(x)"),
                None => "Fichier illisible".to_string(),
            };
            interne("↺", format!("Restaurer : {}", sauvegarde::libelle_date(s.date)), detail, Interne::Restaurer(i))
        }));
        e
    }

    fn elements_maj(&self) -> Vec<Element> {
        use super::super::mises_a_jour::EtatMaj;
        let actif = self.bib.config.verifier_maj;
        let mut e = vec![info(format!("Version installée : {}", maj::VERSION_ACTUELLE), "")];
        match &self.maj {
            EtatMaj::Inconnu => {}
            EtatMaj::Recherche { .. } => e.push(info("Recherche en cours…", "")),
            EtatMaj::AJour => e.push(info("Le launcher est à jour", "")),
            EtatMaj::Erreur(m) => e.push(info(m.clone(), "")),
            EtatMaj::Disponible(p) => e.push(interne(
                "⬆",
                format!("Version {} disponible", p.version),
                "Ouvre la page de téléchargement",
                Interne::OuvrirPageMaj(p.url.clone()),
            )),
        }
        e.push(action("🔄", "Vérifier maintenant", "", Action::VerifierMaj));
        e.push(interne(coche(actif), "Vérifier au démarrage", etat(actif), Interne::Reglage(Reglage::VerifierMaj)));
        e
    }

    fn elements_stats(&self) -> Vec<Element> {
        let jeux = &self.bib.config.jeux;
        let total: u64 = jeux.iter().map(|j| j.temps_jeu).sum();
        let semaine = stats::temps_depuis(jeux, maintenant().saturating_sub(7 * 86400));
        let parties: usize = jeux.iter().map(|j| j.sessions.len()).sum();
        let joues = jeux.iter().filter(|j| j.temps_jeu > 0).count();
        let mut e = vec![
            info("Temps de jeu total", stats::duree(total)),
            info("7 derniers jours", stats::duree(semaine)),
            info("Parties mesurées", parties.to_string()),
            info("Jeux joués", format!("{joues} / {}", jeux.len())),
        ];
        if let Some(jour) = stats::date_locale(maintenant()) {
            let activite = stats::activite_par_jour(jeux, jour, 14);
            let quatorze: u64 = activite.iter().map(|(_, s)| *s).sum();
            e.push(info("14 derniers jours", stats::duree(quatorze)));
        }
        e.extend(stats::plus_joues(jeux, 10).into_iter().filter_map(|(id, s)| {
            let j = self.bib.jeu(id)?;
            Some(info(format!("Plus joué : {}", j.nom), stats::duree(s)))
        }));
        e.extend(
            stats::temps_par_tag(jeux)
                .into_iter()
                .map(|(t, s)| info(format!("Tag : {t}"), stats::duree(s))),
        );
        let jamais = jeux.iter().filter(|j| j.dernier_lancement.is_none()).count();
        e.push(info("Jeux jamais lancés", jamais.to_string()));
        e
    }

    fn elements_fichiers(&mut self, but: &But, dossier: Option<&Path>) -> Vec<Element> {
        let extensions = but.extensions();
        let entrees: Vec<EntreeFichier> = match self.xmb.cache_fichiers(dossier) {
            Some(e) => e,
            None => {
                let e = match dossier {
                    Some(d) => fichiers::lister(d, extensions),
                    None => fichiers::lecteurs(),
                };
                self.xmb.memoriser_fichiers(dossier, e.clone());
                e
            }
        };
        let mut liste = Vec::new();
        if let (true, Some(d)) = (but.cherche_un_dossier(), dossier) {
            let titre = match but {
                But::Exporter => "Exporter ici",
                _ => "Scanner ce dossier",
            };
            liste.push(interne("✔", titre, d.display().to_string(), Interne::Selection(but.clone(), d.to_path_buf())));
        }
        liste.extend(entrees.into_iter().map(|e| {
            let (glyphe, effet) = if e.dossier {
                ("🗁", Interne::Entrer(but.clone(), e.chemin))
            } else {
                ("▶", Interne::Selection(but.clone(), e.chemin))
            };
            interne(glyphe, e.nom, "", effet)
        }));
        if liste.is_empty() {
            liste.push(info("Rien à afficher ici", ""));
        }
        liste
    }

    fn elements_revue(&self) -> Vec<Element> {
        let Some(revue) = &self.revue else {
            return Vec::new();
        };
        let n = revue.candidats.iter().filter(|c| c.coche).count();
        let mut e = vec![
            interne("✔", format!("Importer {n} jeu(x)"), "", Interne::ImporterRevue),
            interne("●", "Tout cocher", "", Interne::ToutCocher(true)),
            interne("○", "Tout décocher", "", Interne::ToutCocher(false)),
            interne("●", "Uniquement les principaux", "", Interne::Principaux),
            interne("×", "Annuler", "", Interne::AnnulerRevue),
        ];
        e.extend(revue.candidats.iter().enumerate().map(|(i, c)| {
            let detail = if c.doublon {
                "Déjà dans la bibliothèque".to_string()
            } else {
                format!("{}  ·  Options pour renommer", c.groupe)
            };
            let effet = if c.doublon { Effet::Aucun } else { Effet::Interne(Interne::BasculerCandidat(i)) };
            el(coche(c.coche), c.nom.clone(), detail, effet)
        }));
        e
    }

    fn elements_dialogue(&self) -> Vec<Element> {
        let nom = |id: u64| self.bib.jeu(id).map(|j| j.nom.clone()).unwrap_or_default();
        let annuler = interne("×", "Annuler", "", Interne::FermerDialogue);
        match &self.dialogue {
            Some(Dialogue::Retirer(id)) => vec![
                info(format!("Retirer « {} » ?", nom(*id)), "Le jeu reste installé sur votre ordinateur"),
                interne("➖", "Retirer", "", Interne::ConfirmerRetrait(*id)),
                annuler,
            ],
            Some(Dialogue::Supprimer { id, cible }) => vec![
                info(format!("Supprimer « {} » ?", nom(*id)), format!("{} part à la corbeille", cible.display())),
                interne("🗑", "Mettre à la corbeille", "", Interne::ConfirmerSuppression(*id, cible.clone())),
                annuler,
            ],
            Some(Dialogue::SuppressionRefusee { id, raison }) => vec![
                info(format!("Suppression de « {} » impossible", nom(*id)), raison.clone()),
                interne("➖", "Retirer seulement du launcher", "", Interne::ConfirmerRetrait(*id)),
                action("🗁", "Ouvrir l'emplacement", "", Action::OuvrirEmplacement(*id)),
                annuler,
            ],
            Some(Dialogue::Introuvable(id)) => vec![
                info(format!("« {} » est introuvable", nom(*id)), "L'exécutable n'existe plus"),
                page("🗁", "Localiser l'exécutable", "", Page::Fichiers(But::Localiser(*id), None)),
                interne("➖", "Retirer du launcher", "", Interne::ConfirmerRetrait(*id)),
                annuler,
            ],
            Some(Dialogue::SupprimerTag(t)) => {
                let n = self.bib.config.jeux.iter().filter(|j| j.a_tag(t)).count();
                vec![
                    info(format!("Retirer le tag « {t} » de {n} jeu(x) ?"), "Les jeux eux-mêmes ne sont pas touchés"),
                    interne("🗑", "Supprimer le tag", "", Interne::ConfirmerSuppressionTag(t.clone())),
                    annuler,
                ]
            }
            Some(Dialogue::RetirerIntrouvables) => vec![
                info(format!("Retirer {} jeu(x) introuvable(s) ?", self.bib.ids_introuvables().len()), ""),
                interne("➖", "Retirer", "", Interne::ConfirmerIntrouvables),
                annuler,
            ],
            Some(Dialogue::Restaurer(s)) => vec![
                info(format!("Restaurer la sauvegarde du {} ?", sauvegarde::libelle_date(s.date)), "Une copie de l'état actuel est faite avant"),
                interne("↺", "Restaurer", "", Interne::ConfirmerRestauration(s.chemin.clone())),
                annuler,
            ],
            Some(Dialogue::Importer { config, fichier }) => vec![
                info(format!("« {fichier} » : {} jeu(x)", config.jeux.len()), ""),
                interne("+", "Ajouter les jeux manquants", "", Interne::ImportFusion),
                interne("↺", "Tout remplacer", "Bibliothèque et réglages", Interne::ImportRemplacer),
                annuler,
            ],
            _ => vec![annuler],
        }
    }

    /// Titre affiché au-dessus d'une sous-page.
    pub(super) fn titre_page_xmb(&self, p: &Page) -> String {
        let nom = |id: &u64| self.bib.jeu(*id).map(|j| j.nom.clone()).unwrap_or_default();
        match p {
            Page::Collection(t) => t.clone(),
            Page::ParStatut(s) => s.libelle().to_string(),
            Page::Recherche => format!("Recherche : {}", self.recherche),
            Page::OptionsJeu(id) => nom(id),
            Page::StatutJeu(id) => format!("{} : statut", nom(id)),
            Page::TagsJeu(id) => format!("{} : tags", nom(id)),
            Page::ImagesJeu(id) => format!("{} : images", nom(id)),
            Page::Tri => "Trier par".to_string(),
            Page::Bibliotheque => "Bibliothèque".to_string(),
            Page::Apparence => "Apparence".to_string(),
            Page::Themes => "Thème".to_string(),
            Page::Accents => "Couleur d'accent".to_string(),
            Page::TaillesTuile => "Taille des tuiles".to_string(),
            Page::PleinEcran => "Plein écran et manette".to_string(),
            Page::StyleEcran => "Style du plein écran".to_string(),
            Page::ChoixGlyphes => "Boutons affichés".to_string(),
            Page::Images => "Jaquettes et bannières".to_string(),
            Page::Tags => "Tags".to_string(),
            Page::OptionsTag(t) => t.clone(),
            Page::Sauvegardes => "Sauvegardes".to_string(),
            Page::Donnees => "Données".to_string(),
            Page::MisesAJour => "Mises à jour".to_string(),
            Page::Statistiques => "Statistiques".to_string(),
            Page::Astuces => "Astuces".to_string(),
            Page::Fichiers(_, Some(d)) => d.display().to_string(),
            Page::Fichiers(_, None) => "Choisir un lecteur".to_string(),
            Page::Revue => "Jeux trouvés".to_string(),
            Page::Dialogue => "Confirmation".to_string(),
        }
    }
}
