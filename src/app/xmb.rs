//! Menu plein écran façon XMB : une barre horizontale de catégories et une
//! liste verticale. Tout s'utilise à la manette, sans souris ni clavier.

mod clavier;
mod fichiers;
mod fond;
mod pages;

use super::*;
use crate::config::{Glyphes, StylePleinEcran};
use crate::manette::FamilleManette;
use crate::sons::{self, Son};
use clavier::{Clavier, IssueClavier};
use fichiers::EntreeFichier;
use pages::{But, Effet, Element, Interne, Page, Reglage, Saisie, Visuel};
use std::path::Path;

const CATEGORIES: [Categorie; 6] = [
    Categorie::Jeux,
    Categorie::Recents,
    Categorie::Favoris,
    Categorie::Collections,
    Categorie::Parametres,
    Categorie::Quitter,
];
const INDEX_PARAMETRES: usize = 4;
/// Dans la revue d'un scan, nombre d'entrées avant les jeux trouvés.
const ACTIONS_REVUE: usize = 5;

const ECART_CATEGORIES: f32 = 170.0;
const HAUTEUR_ELEMENT: f32 = 76.0;
/// Raideur du défilement : un appui s'installe en un peu plus de 200 ms.
const RAIDEUR_DEFILEMENT: f32 = 18.0;
/// En deçà de ces écarts, le défilement est considéré comme arrivé.
const ECART_ARRIVE: f32 = 0.002;
const VITESSE_ARRIVEE: f32 = 0.02;
/// Taille des jaquettes voisines par rapport à celle mise en avant.
const ECHELLE_VOISINE: f32 = 0.58;
const ECART_JAQUETTES: f32 = 28.0;
/// Hauteur plancher d'une jaquette, sur une toute petite fenêtre.
const HAUTEUR_JAQUETTE_MIN: f32 = 240.0;
/// Place réservée sous la jaquette pour le nom et les détails du jeu.
const HAUTEUR_LEGENDE: f32 = 86.0;
const HAUTEUR_BARRE_AIDE: f32 = 64.0;
/// Les vagues sont lentes : 30 images par seconde suffisent et ménagent le processeur.
const INTERVALLE_VAGUES: Duration = Duration::from_millis(33);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Categorie {
    Jeux,
    Recents,
    Favoris,
    Collections,
    Parametres,
    Quitter,
}

impl Categorie {
    fn libelle(self) -> &'static str {
        match self {
            Categorie::Jeux => "Jeux",
            Categorie::Recents => "Récents",
            Categorie::Favoris => "Favoris",
            Categorie::Collections => "Collections",
            Categorie::Parametres => "Paramètres",
            Categorie::Quitter => "Quitter",
        }
    }

    fn icone(self) -> &'static str {
        match self {
            Categorie::Jeux => "🎮",
            Categorie::Recents => "🕑",
            Categorie::Favoris => "⭐",
            Categorie::Collections => "🏷",
            Categorie::Parametres => "⚙",
            Categorie::Quitter => "×",
        }
    }
}

struct Niveau {
    page: Page,
    focus: usize,
}

#[derive(Default)]
pub(super) struct EtatXmb {
    categorie: usize,
    /// Élément focalisé de chaque catégorie.
    focus: [usize; CATEGORIES.len()],
    /// Sous-pages ouvertes, la dernière étant affichée.
    pile: Vec<Niveau>,
    clavier: Option<Clavier>,
    /// Jeu à retrouver dès que possible (fin de partie).
    focus_voulu: Option<u64>,
    /// Dernier dossier lu, pour ne pas relire le disque à chaque image.
    fichiers: Option<(Option<PathBuf>, Vec<EntreeFichier>)>,
    /// Position affichée de la liste, qui rejoint le focus en continu.
    pos_liste: Lisseur,
    pos_categorie: Lisseur,
    /// Liste affichée : en changer replace la position sans animation.
    cle_liste: (usize, usize),
}

impl EtatXmb {
    pub(super) fn retrouver_jeu(&mut self, id: u64) {
        self.focus_voulu = Some(id);
    }

    fn focus_courant(&self) -> usize {
        match self.pile.last() {
            Some(n) => n.focus,
            None => self.focus[self.categorie],
        }
    }

    fn definir_focus(&mut self, focus: usize) {
        match self.pile.last_mut() {
            Some(n) => n.focus = focus,
            None => self.focus[self.categorie] = focus,
        }
    }

    fn changer_categorie(&mut self, delta: isize) {
        let n = CATEGORIES.len() as isize;
        self.categorie = (self.categorie as isize + delta).rem_euclid(n) as usize;
        self.pile.clear();
    }

    fn ouvrir(&mut self, page: Page) {
        self.pile.push(Niveau { page, focus: 0 });
    }

    /// Les confirmations et la revue d'un scan restent dans la pile : une page
    /// peut s'ouvrir par-dessus (choisir un fichier pour localiser un jeu).
    fn synchroniser(&mut self, dialogue: bool, revue: bool) {
        self.pile.retain(|n| match n.page {
            Page::Dialogue => dialogue,
            Page::Revue => revue,
            _ => true,
        });
        let present = |e: &Self, page: Page| e.pile.iter().any(|n| n.page == page);
        if revue && !present(self, Page::Revue) {
            self.ouvrir(Page::Revue);
        }
        if dialogue && !present(self, Page::Dialogue) {
            self.ouvrir(Page::Dialogue);
        }
    }

    /// Une confirmation ou une revue attend une réponse explicite.
    fn page_bloquante(&self) -> bool {
        self.pile
            .iter()
            .any(|n| matches!(n.page, Page::Dialogue | Page::Revue))
    }

    fn cache_fichiers(&self, dossier: Option<&Path>) -> Option<Vec<EntreeFichier>> {
        let (cle, entrees) = self.fichiers.as_ref()?;
        (cle.as_deref() == dossier).then(|| entrees.clone())
    }

    fn memoriser_fichiers(&mut self, dossier: Option<&Path>, entrees: Vec<EntreeFichier>) {
        self.fichiers = Some((dossier.map(Path::to_path_buf), entrees));
    }
}

/// Nouvelle position après un déplacement, bloquée aux extrémités.
fn deplacer_focus(focus: usize, n: usize, delta: isize) -> usize {
    if n == 0 {
        return 0;
    }
    (focus as isize + delta).clamp(0, n as isize - 1) as usize
}

/// Position qui rejoint sa cible comme un ressort à amortissement critique.
/// Quand une direction est maintenue, les répétitions de la manette deviennent
/// une vitesse régulière : une animation à durée fixe repartirait à chacune
/// d'elles, et un lissage simple garderait des à-coups.
#[derive(Debug, Default, Clone, Copy)]
struct Lisseur {
    pos: f32,
    vitesse: f32,
}

impl Lisseur {
    fn a(pos: f32) -> Self {
        Self { pos, vitesse: 0.0 }
    }

    /// Solution exacte du ressort : stable quelle que soit la durée de l'image.
    fn avancer(&mut self, cible: f32, dt: f32) {
        let amorti = (-RAIDEUR_DEFILEMENT * dt).exp();
        let ecart = self.pos - cible;
        let terme = (self.vitesse + RAIDEUR_DEFILEMENT * ecart) * dt;
        self.vitesse = (self.vitesse - RAIDEUR_DEFILEMENT * terme) * amorti;
        self.pos = cible + (ecart + terme) * amorti;
        if (cible - self.pos).abs() < ECART_ARRIVE && self.vitesse.abs() < VITESSE_ARRIVEE {
            *self = Self::a(cible);
        }
    }
}

/// Une liste de jeux défile à l'horizontale ; les menus restent verticaux.
fn est_carrousel(elements: &[Element]) -> bool {
    !elements.is_empty() && elements.iter().all(|e| matches!(e.visuel, Visuel::Jeu(_)))
}

/// Décalage horizontal et échelle d'une jaquette, selon sa distance au focus.
fn place_carrousel(distance: f32, largeur: f32) -> (f32, f32) {
    let proche = distance.abs().min(1.0);
    let echelle = melange(1.0, ECHELLE_VOISINE, proche);
    let voisine = largeur * ECHELLE_VOISINE;
    let premier_pas = (largeur + voisine) / 2.0 + ECART_JAQUETTES;
    let decalage = melange(0.0, premier_pas, proche)
        + (distance.abs() - 1.0).max(0.0) * (voisine + ECART_JAQUETTES);
    (decalage * distance.signum(), echelle)
}

/// Lit une couleur « ff8800 » ou « #FF8800 ».
fn analyser_couleur(texte: &str) -> Option<[u8; 3]> {
    let t = texte.trim().trim_start_matches('#');
    if t.len() != 6 || !t.is_ascii() {
        return None;
    }
    let canal = |i: usize| u8::from_str_radix(&t[i..i + 2], 16).ok();
    Some([canal(0)?, canal(2)?, canal(4)?])
}

fn melange(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[derive(Clone, Copy)]
enum Bouton {
    A,
    B,
    X,
    Y,
    Gachettes,
    Start,
    Direction,
}

/// Dessine le glyphe d'un bouton et renvoie sa largeur.
fn glyphe(peintre: &egui::Painter, centre: egui::Pos2, b: Bouton, famille: Option<FamilleManette>) -> f32 {
    let p = theme::p();
    let police = FontId::new(theme::POLICE_LEGENDE, theme::gras());
    let rayon = 13.0;
    let texte_pilule = |t: &str| {
        let galley = peintre.layout_no_wrap(t.to_string(), police.clone(), p.fg);
        let largeur = galley.size().x + 20.0;
        let r = egui::Rect::from_center_size(
            centre + Vec2::new(largeur / 2.0 - rayon, 0.0),
            Vec2::new(largeur, 26.0),
        );
        peintre.rect_filled(r, CornerRadius::same(theme::RAYON_S), p.bg_btn_hover);
        peintre.galley(r.center() - galley.size() / 2.0, galley, p.fg);
        largeur
    };
    match (famille, b) {
        (None, Bouton::Direction) => texte_pilule("Flèches"),
        (Some(_), Bouton::Direction) => texte_pilule("Stick"),
        (None, Bouton::A) => texte_pilule("Entrée"),
        (None, Bouton::B) => texte_pilule("Échap"),
        (None, Bouton::X) => texte_pilule("R"),
        (None, Bouton::Y) => texte_pilule("F"),
        (None, Bouton::Gachettes) => texte_pilule("Q / E"),
        (None, Bouton::Start) => texte_pilule("P"),
        (Some(FamilleManette::Xbox), Bouton::Gachettes) => texte_pilule("LB / RB"),
        (Some(FamilleManette::Xbox), Bouton::Start) => texte_pilule("Menu"),
        (Some(FamilleManette::PlayStation), Bouton::Gachettes) => texte_pilule("L1 / R1"),
        (Some(FamilleManette::PlayStation), Bouton::Start) => texte_pilule("Options"),
        (Some(FamilleManette::Xbox), lettre) => {
            let (t, couleur) = match lettre {
                Bouton::A => ("A", Color32::from_rgb(96, 176, 72)),
                Bouton::B => ("B", Color32::from_rgb(220, 70, 60)),
                Bouton::X => ("X", Color32::from_rgb(64, 130, 220)),
                _ => ("Y", Color32::from_rgb(230, 180, 40)),
            };
            peintre.circle_filled(centre, rayon, couleur);
            peintre.text(centre, Align2::CENTER_CENTER, t, police, Color32::WHITE);
            2.0 * rayon
        }
        (Some(FamilleManette::PlayStation), forme) => {
            peintre.circle_filled(centre, rayon, p.bg_btn_hover);
            let trait_ = |c: Color32| Stroke::new(2.0, c);
            let d = 6.0;
            match forme {
                Bouton::A => {
                    let s = trait_(Color32::from_rgb(120, 170, 240));
                    peintre.line_segment([centre + Vec2::splat(-d), centre + Vec2::splat(d)], s);
                    peintre.line_segment([centre + Vec2::new(-d, d), centre + Vec2::new(d, -d)], s);
                }
                Bouton::B => {
                    peintre.circle_stroke(centre, d, trait_(Color32::from_rgb(235, 90, 90)));
                }
                Bouton::X => {
                    let r = egui::Rect::from_center_size(centre, Vec2::splat(2.0 * d));
                    let s = trait_(Color32::from_rgb(220, 130, 200));
                    peintre.rect_stroke(r, CornerRadius::ZERO, s, egui::StrokeKind::Middle);
                }
                _ => {
                    let points = vec![
                        centre + Vec2::new(0.0, -d),
                        centre + Vec2::new(d, d * 0.8),
                        centre + Vec2::new(-d, d * 0.8),
                    ];
                    peintre.add(egui::Shape::closed_line(points, trait_(Color32::from_rgb(90, 200, 170))));
                }
            }
            2.0 * rayon
        }
    }
}

impl App {
    pub(super) fn style_xmb(&self) -> bool {
        self.bib.config.style_plein_ecran == StylePleinEcran::Xmb
    }

    fn son(&self, son: Son) {
        if self.bib.config.sons {
            sons::jouer(son);
        }
    }

    pub(super) fn famille_glyphes(&self) -> Option<FamilleManette> {
        match self.bib.config.glyphes {
            Glyphes::Xbox => Some(FamilleManette::Xbox),
            Glyphes::PlayStation => Some(FamilleManette::PlayStation),
            Glyphes::Auto => self.manette_connectee.as_ref().map(|(_, f)| *f),
        }
    }

    fn elements_courants(&mut self) -> Vec<Element> {
        match self.xmb.pile.last().map(|n| n.page.clone()) {
            Some(page) => self.elements_page(&page),
            None => self.elements_categorie(CATEGORIES[self.xmb.categorie]),
        }
    }

    /// Les confirmations et la revue d'un scan deviennent des pages du menu.
    fn synchroniser_xmb(&mut self) {
        self.xmb.synchroniser(self.dialogue.is_some(), self.revue.is_some());
        // Un jeu retiré emporte les pages qui le concernent.
        let jeux = &self.bib;
        self.xmb.pile.retain(|n| match n.page {
            Page::OptionsJeu(id)
            | Page::StatutJeu(id)
            | Page::TagsJeu(id)
            | Page::ImagesJeu(id) => jeux.jeu(id).is_some(),
            _ => true,
        });
    }

    /// Quitte toutes les pages ouvertes : leur état ne doit pas survivre.
    pub(super) fn sortir_pages(&mut self) {
        let en_recherche = self.xmb.pile.iter().any(|n| n.page == Page::Recherche);
        if en_recherche {
            self.recherche.clear();
        }
        if !self.edition_visuels.as_ref().is_some_and(|e| e.occupe()) {
            self.edition_visuels = None;
        }
        self.xmb.pile.clear();
        self.xmb.clavier = None;
        self.xmb.fichiers = None;
    }

    /// Replace le focus sur un jeu après une partie.
    fn appliquer_focus_voulu(&mut self, elements: &[Element]) -> bool {
        let Some(id) = self.xmb.focus_voulu else {
            return false;
        };
        if let Some(i) = elements.iter().position(|e| e.visuel == Visuel::Jeu(id)) {
            self.xmb.definir_focus(i);
            self.xmb.focus_voulu = None;
            return false;
        }
        if self.dialogue.is_some() || self.revue.is_some() {
            return false;
        }
        // Le jeu n'est pas dans cette liste : retour à la liste complète.
        self.xmb.focus_voulu = None;
        if self.bib.jeu(id).is_none() {
            return false;
        }
        self.xmb.pile.clear();
        self.xmb.categorie = 0;
        self.xmb.focus_voulu = Some(id);
        true
    }

    fn fermer_page(&mut self) {
        match self.xmb.pile.last().map(|n| n.page.clone()) {
            Some(Page::Dialogue) => self.dialogue = None,
            Some(Page::Revue) => self.revue = None,
            Some(Page::Recherche) => {
                self.recherche.clear();
                self.xmb.pile.pop();
            }
            Some(Page::ImagesJeu(_)) => {
                if !self.edition_visuels.as_ref().is_some_and(|e| e.occupe()) {
                    self.edition_visuels = None;
                }
                self.xmb.pile.pop();
            }
            Some(_) => {
                self.xmb.pile.pop();
            }
            None => {}
        }
    }

    /// Ferme les navigateurs de fichiers ouverts.
    fn fermer_fichiers(&mut self) {
        while matches!(self.xmb.pile.last(), Some(Niveau { page: Page::Fichiers(..), .. })) {
            self.xmb.pile.pop();
        }
        self.xmb.fichiers = None;
    }

    pub(super) fn commande_xmb(&mut self, c: Commande, ctx: &egui::Context, actions: &mut Vec<Action>) {
        if let Some(clavier) = &mut self.xmb.clavier {
            let issue = clavier.commande(c);
            match issue {
                IssueClavier::Continuer => self.son(Son::Deplacement),
                IssueClavier::Annuler => {
                    self.xmb.clavier = None;
                    self.son(Son::Retour);
                }
                IssueClavier::Valider => {
                    if let Some(clavier) = self.xmb.clavier.take() {
                        self.son(Son::Validation);
                        self.appliquer_saisie(clavier.saisie, clavier.texte, ctx);
                    }
                }
            }
            return;
        }
        // Scan ou récupération en cours : seul Retour agit, pour annuler.
        if self.scan.is_some() || self.lot_en_cours() {
            if c == Commande::Retour {
                self.annuler_taches();
            }
            return;
        }

        self.synchroniser_xmb();
        let elements = self.elements_courants();
        let focus = self.xmb.focus_courant().min(elements.len().saturating_sub(1));
        let a_pile = !self.xmb.pile.is_empty();
        let bloque = self.xmb.page_bloquante();
        let bouger = |etat: &mut EtatXmb, delta| {
            let nouveau = deplacer_focus(focus, elements.len(), delta);
            etat.definir_focus(nouveau);
            nouveau != focus
        };
        let carrousel = est_carrousel(&elements);
        match c {
            Commande::Gauche | Commande::Droite if carrousel => {
                let delta = if c == Commande::Gauche { -1 } else { 1 };
                if bouger(&mut self.xmb, delta) {
                    self.son(Son::Deplacement);
                }
            }
            Commande::Haut | Commande::Bas if carrousel => {
                if !a_pile {
                    self.xmb.changer_categorie(if c == Commande::Haut { -1 } else { 1 });
                    self.son(Son::Deplacement);
                }
            }
            Commande::Haut | Commande::Bas => {
                let delta = if c == Commande::Haut { -1 } else { 1 };
                if bouger(&mut self.xmb, delta) {
                    self.son(Son::Deplacement);
                }
            }
            Commande::Gauche | Commande::Droite if !a_pile => {
                self.xmb.changer_categorie(if c == Commande::Gauche { -1 } else { 1 });
                self.son(Son::Deplacement);
            }
            Commande::Gauche if !bloque => {
                self.fermer_page();
                self.son(Son::Retour);
            }
            Commande::Gauche | Commande::Droite => {}
            Commande::FiltrePrecedent | Commande::FiltreSuivant | Commande::Parametres if bloque => {}
            Commande::FiltrePrecedent | Commande::FiltreSuivant => {
                let delta = if c == Commande::FiltrePrecedent { -1 } else { 1 };
                self.sortir_pages();
                self.xmb.changer_categorie(delta);
                self.son(Son::Deplacement);
            }
            Commande::Valider => {
                if let Some(e) = elements.get(focus).cloned() {
                    if !matches!(e.effet, Effet::Aucun) {
                        self.son(Son::Validation);
                    }
                    self.appliquer_effet(e.effet, ctx, actions);
                }
            }
            Commande::Retour => {
                if a_pile {
                    self.fermer_page();
                    self.son(Son::Retour);
                }
            }
            Commande::Options => self.ouvrir_options(&elements, focus),
            Commande::Recherche => {
                self.xmb.clavier = Some(Clavier::nouveau(
                    "Rechercher un jeu",
                    self.recherche.clone(),
                    Saisie::Recherche,
                ));
                self.son(Son::Validation);
            }
            Commande::Parametres => {
                self.sortir_pages();
                self.xmb.categorie = INDEX_PARAMETRES;
                self.son(Son::Validation);
            }
            Commande::BasculerPleinEcran => {}
        }
    }

    fn ouvrir_options(&mut self, elements: &[Element], focus: usize) {
        let page = self.xmb.pile.last().map(|n| n.page.clone());
        if page == Some(Page::Revue) {
            if let Some(i) = focus.checked_sub(ACTIONS_REVUE)
                && let Some(c) = self.revue.as_ref().and_then(|r| r.candidats.get(i))
                && !c.doublon
            {
                let nom = c.nom.clone();
                self.xmb.clavier = Some(Clavier::nouveau("Nom du jeu", nom, Saisie::NomCandidat(i)));
            }
            return;
        }
        if let Some(Element { visuel: Visuel::Jeu(id), .. }) = elements.get(focus)
            && !matches!(page, Some(Page::OptionsJeu(_)))
        {
            self.xmb.ouvrir(Page::OptionsJeu(*id));
            self.son(Son::Validation);
        }
    }

    fn appliquer_effet(&mut self, effet: Effet, ctx: &egui::Context, actions: &mut Vec<Action>) {
        match effet {
            Effet::Aucun => {}
            Effet::Action(a) => actions.push(a),
            Effet::Ouvrir(page) => {
                if let Page::ImagesJeu(id) = page {
                    self.ouvrir_visuels(id);
                }
                self.xmb.fichiers = None;
                self.xmb.ouvrir(page);
            }
            Effet::Saisir(titre, initial, saisie) => {
                self.xmb.clavier = Some(Clavier::nouveau(titre, initial, saisie));
            }
            Effet::Interne(i) => self.appliquer_interne(i, ctx),
        }
    }

    fn basculer_reglage(&mut self, r: Reglage) {
        let c = &mut self.bib.config;
        let champ = match r {
            Reglage::Recents => &mut c.afficher_recents,
            Reglage::FermerFin => &mut c.fermer_apres_lancement,
            Reglage::FondAnime => &mut c.fond_anime,
            Reglage::Sons => &mut c.sons,
            Reglage::DemarrerPleinEcran => &mut c.demarrer_plein_ecran,
            Reglage::BasculeManette => &mut c.bascule_manette,
            Reglage::VerifierMaj => &mut c.verifier_maj,
        };
        *champ = !*champ;
        self.sauver();
    }

    fn appliquer_selection(&mut self, but: But, chemin: PathBuf, ctx: &egui::Context) {
        self.fermer_fichiers();
        match but {
            But::Ajouter => self.ajouter_exes(vec![chemin]),
            But::Scanner => self.lancer_scan(chemin, ctx),
            But::Localiser(id) => {
                self.dialogue = None;
                self.adopter_exe(id, chemin);
            }
            But::Icone(id) => self.adopter_icone(id, &chemin),
            But::Image(id, genre) => self.importer_visuel(id, genre, &chemin),
            But::Exporter => {
                let date = chrono::Local::now().format("%Y-%m-%d").to_string();
                self.exporter_vers(&chemin_export_libre(&chemin, &date));
            }
            But::Importer => self.lire_import(&chemin),
        }
    }

    fn appliquer_interne(&mut self, i: Interne, ctx: &egui::Context) {
        match i {
            Interne::Reglage(r) => self.basculer_reglage(r),
            Interne::Accent(c) => {
                self.bib.config.accent_perso = c;
                self.appliquer_theme(ctx);
                self.sauver();
            }
            Interne::Style(s) => {
                self.bib.config.style_plein_ecran = s;
                self.sauver();
            }
            Interne::Glyphes(g) => {
                self.bib.config.glyphes = g;
                self.sauver();
            }
            Interne::Taille(t) => {
                self.bib.config.taille_tuile = t;
                self.sauver();
            }
            Interne::Statut(id, s) => {
                self.bib.definir_statut(id, s);
                self.sauver();
            }
            Interne::BasculerTag(id, t) => {
                if let Some(j) = self.bib.config.jeu_mut(id)
                    && !j.ajouter_tag(&t)
                {
                    j.retirer_tag(&t);
                }
                self.sauver();
            }
            Interne::SupprimerTag(t) => self.dialogue = Some(Dialogue::SupprimerTag(t)),
            Interne::ConfirmerSuppressionTag(t) => {
                self.dialogue = None;
                self.bib.config.supprimer_tag(&t);
                self.sauver();
                self.xmb.pile.retain(|n| !matches!(n.page, Page::OptionsTag(_)));
            }
            Interne::RetirerImage(id, genre) => {
                self.bib.changer_visuel(id, genre, None);
                self.icones.invalider_genre(id, genre);
                self.sauver();
            }
            Interne::Telecharger(id_sgdb) => self.lancer_telechargement(id_sgdb, ctx),
            Interne::OuvrirSauvegardes => {
                self.rafraichir_sauvegardes();
                self.xmb.ouvrir(Page::Sauvegardes);
            }
            Interne::OuvrirPageMaj(url) => self.ouvrir_page_maj(&url),
            Interne::Restaurer(i) => {
                if let Some(s) = self.sauvegardes.get(i).cloned() {
                    self.dialogue = Some(Dialogue::Restaurer(s));
                }
            }
            Interne::Selection(but, chemin) => self.appliquer_selection(but, chemin, ctx),
            Interne::Entrer(but, chemin) => {
                self.xmb.fichiers = None;
                self.xmb.ouvrir(Page::Fichiers(but, Some(chemin)));
            }
            Interne::ToutCocher(valeur) => {
                if let Some(r) = &mut self.revue {
                    r.candidats.iter_mut().for_each(|c| c.coche = valeur && !c.doublon);
                }
            }
            Interne::Principaux => {
                if let Some(r) = &mut self.revue {
                    cocher_principaux(&mut r.candidats);
                }
            }
            Interne::BasculerCandidat(i) => {
                if let Some(c) = self.revue.as_mut().and_then(|r| r.candidats.get_mut(i)) {
                    c.coche = !c.coche;
                }
            }
            Interne::ImporterRevue => {
                let candidats = self.revue.take().map(|r| r.candidats).unwrap_or_default();
                self.importer_revue(candidats);
            }
            Interne::AnnulerRevue => self.revue = None,
            Interne::ConfirmerRetrait(id) => {
                self.dialogue = None;
                self.retirer(id);
            }
            Interne::ConfirmerSuppression(id, cible) => {
                self.dialogue = None;
                self.supprimer(id, cible, ctx);
            }
            Interne::ConfirmerIntrouvables => {
                self.dialogue = None;
                for id in self.bib.ids_introuvables() {
                    self.retirer(id);
                }
            }
            Interne::ConfirmerRestauration(chemin) => {
                self.dialogue = None;
                match sauvegarde::lire(&chemin) {
                    Ok(c) => {
                        self.remplacer_config(c, ctx);
                        self.notifier("Sauvegarde restaurée.");
                    }
                    Err(e) => self.notifier_erreur(e),
                }
            }
            Interne::ImportFusion => {
                if let Some(Dialogue::Importer { config, .. }) = self.dialogue.take() {
                    let n = self.bib.config.fusionner(*config);
                    self.sauver();
                    self.bib.verifier_manquants();
                    self.notifier(format!("{n} jeu(x) importé(s)."));
                }
            }
            Interne::ImportRemplacer => {
                if let Some(Dialogue::Importer { config, .. }) = self.dialogue.take() {
                    self.remplacer_config(*config, ctx);
                    self.notifier("Bibliothèque importée.");
                }
            }
            Interne::FermerDialogue => self.dialogue = None,
            Interne::ModeBureau => self.basculer_mode_tv(ctx),
        }
    }

    fn appliquer_saisie(&mut self, saisie: Saisie, texte: String, ctx: &egui::Context) {
        match saisie {
            Saisie::Recherche => {
                self.recherche = texte.trim().to_string();
                let deja = matches!(self.xmb.pile.last(), Some(Niveau { page: Page::Recherche, .. }));
                if self.recherche.is_empty() {
                    if deja {
                        self.xmb.pile.pop();
                    }
                } else if !deja {
                    self.xmb.ouvrir(Page::Recherche);
                }
                return;
            }
            Saisie::Renommer(id) => {
                self.bib.renommer(id, &texte);
            }
            Saisie::Notes(id) => self.bib.definir_notes(id, &texte),
            Saisie::Arguments(id) => self.bib.definir_arguments(id, &texte),
            Saisie::NouveauTag(id) => {
                if let Some(j) = self.bib.config.jeu_mut(id) {
                    j.ajouter_tag(&texte);
                }
            }
            Saisie::RenommerTag(ancien) => {
                self.bib.config.renommer_tag(&ancien, &texte);
                self.xmb.pile.pop();
            }
            Saisie::CleApi => {
                self.cle_sgdb = texte;
                self.enregistrer_cle();
                return;
            }
            Saisie::RechercheImages(_) => {
                if let Some(e) = &mut self.edition_visuels {
                    e.definir_recherche(texte);
                }
                self.lancer_recherche(ctx);
                return;
            }
            Saisie::NomCandidat(i) => {
                if let Some(c) = self.revue.as_mut().and_then(|r| r.candidats.get_mut(i)) {
                    c.nom = texte;
                }
                return;
            }
            Saisie::Accent => match analyser_couleur(&texte) {
                Some(c) => {
                    self.bib.config.accent_perso = Some(c);
                    self.appliquer_theme(ctx);
                }
                None => {
                    self.notifier_erreur("Couleur invalide : six chiffres hexadécimaux attendus.");
                    return;
                }
            },
        }
        self.sauver();
    }

    /// Touches physiques pendant que le clavier virtuel est ouvert.
    pub(super) fn clavier_physique_xmb(&mut self, ctx: &egui::Context) -> bool {
        let Some(clavier) = &mut self.xmb.clavier else {
            return false;
        };
        let (texte, commandes) = ctx.input(|i| {
            let texte: String = i
                .events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Text(t) => Some(t.as_str()),
                    _ => None,
                })
                .collect();
            let touches = [
                (Key::ArrowUp, Commande::Haut),
                (Key::ArrowDown, Commande::Bas),
                (Key::ArrowLeft, Commande::Gauche),
                (Key::ArrowRight, Commande::Droite),
                (Key::Enter, Commande::Parametres),
                (Key::Escape, Commande::Retour),
                (Key::Backspace, Commande::Recherche),
            ];
            let commandes: Vec<Commande> = touches
                .iter()
                .filter(|(k, _)| i.key_pressed(*k))
                .map(|(_, c)| *c)
                .collect();
            (texte, commandes)
        });
        clavier.recevoir_texte(&texte);
        self.commandes.extend(commandes);
        true
    }

    fn dessiner_visuel(&mut self, ui: &egui::Ui, peintre: &egui::Painter, e: &Element, zone: egui::Rect, proximite: f32) {
        let p = theme::p();
        match e.visuel {
            Visuel::Jeu(id) => {
                if let Some(jeu) = self.bib.jeu(id).cloned() {
                    self.dessiner_icone(ui, &jeu, zone, self.bib.manquants.contains(&id));
                    let marque = zone.left_top() + Vec2::splat(6.0);
                    marquer_statut(peintre, marque, jeu.statut);
                }
            }
            Visuel::Glyphe(g) => {
                peintre.text(
                    zone.center(),
                    Align2::CENTER_CENTER,
                    g,
                    FontId::proportional(melange(22.0, 34.0, proximite)),
                    if proximite > 0.5 { p.accent } else { p.fg_muted },
                );
            }
        }
    }

    fn dessiner_categories(&self, peintre: &egui::Painter, zone: egui::Rect, pos: f32) {
        let p = theme::p();
        let y = zone.top() + 92.0;
        for (i, c) in CATEGORIES.iter().enumerate() {
            let distance = i as f32 - pos;
            let x = zone.center().x + distance * ECART_CATEGORIES;
            let proximite = (1.0 - distance.abs()).clamp(0.0, 1.0);
            let couleur = if proximite > 0.5 { p.accent } else { p.fg_muted };
            peintre.text(
                egui::pos2(x, y),
                Align2::CENTER_CENTER,
                c.icone(),
                FontId::proportional(melange(30.0, 54.0, proximite)),
                couleur,
            );
            let texte = if proximite > 0.05 { p.fg.gamma_multiply(proximite) } else { p.fg_muted };
            peintre.text(
                egui::pos2(x, y + 44.0),
                Align2::CENTER_CENTER,
                c.libelle(),
                FontId::new(melange(theme::POLICE_CORPS, theme::POLICE_SECTION, proximite), theme::gras()),
                texte,
            );
        }
    }

    fn dessiner_liste(
        &mut self,
        ui: &egui::Ui,
        zone: egui::Rect,
        elements: &[Element],
        focus: usize,
    ) {
        let p = theme::p();
        let pos = self.xmb.pos_liste.pos;
        let peintre = ui.painter_at(zone);
        let x = zone.center().x - 80.0;
        let y_focus = zone.top() + HAUTEUR_ELEMENT * 1.6;
        let largeur_texte = (zone.right() - x - 330.0).max(240.0);

        // Liste virtualisée : seuls les éléments proches du focus sont dessinés.
        let avant = (zone.height() / HAUTEUR_ELEMENT).ceil() as isize;
        let debut = (focus as isize - 3).max(0) as usize;
        let fin = ((focus as isize + avant + 2) as usize).min(elements.len());
        for (i, e) in elements.iter().enumerate().take(fin).skip(debut) {
            let distance = i as f32 - pos;
            let proximite = (1.0 - distance.abs()).clamp(0.0, 1.0);
            let y = y_focus + distance * HAUTEUR_ELEMENT;
            let opacite = if distance < -1.5 { 0.0 } else if distance < 0.0 { 0.45 } else { 1.0 };
            if opacite == 0.0 {
                continue;
            }
            let cote = melange(44.0, 66.0, proximite);
            let zone_visuel =
                egui::Rect::from_center_size(egui::pos2(x - 52.0, y), Vec2::splat(cote));
            if proximite > 0.02 {
                let surbrillance = egui::Rect::from_min_max(
                    egui::pos2(x - 100.0, y - 37.0),
                    egui::pos2(x + largeur_texte, y + 37.0),
                );
                peintre.rect_filled(
                    surbrillance,
                    CornerRadius::same(theme::RAYON_L),
                    p.accent.gamma_multiply(0.10 * proximite),
                );
            }
            self.dessiner_visuel(ui, &peintre, e, zone_visuel, proximite);

            let taille = melange(theme::POLICE_SECTION, theme::POLICE_XMB, proximite);
            let couleur = p.fg.gamma_multiply(opacite * melange(0.65, 1.0, proximite));
            let mut job = egui::text::LayoutJob::simple_singleline(
                e.titre.clone(),
                FontId::new(taille, theme::gras()),
                couleur,
            );
            job.wrap.max_width = largeur_texte;
            job.wrap.max_rows = 1;
            job.wrap.break_anywhere = true;
            job.wrap.overflow_character = Some('…');
            let galley = ui.painter().layout_job(job);
            let hauteur = galley.size().y;
            let detail = match e.visuel {
                Visuel::Jeu(id) if proximite > 0.3 => {
                    self.bib.jeu(id).map(resume_jeu).unwrap_or_default()
                }
                _ => e.detail.clone(),
            };
            let a_detail = proximite > 0.3 && !detail.is_empty();
            let y_titre = if a_detail { y - hauteur + 2.0 } else { y - hauteur / 2.0 };
            peintre.galley(egui::pos2(x, y_titre), galley, couleur);
            if a_detail {
                let mut job = egui::text::LayoutJob::simple_singleline(
                    detail,
                    FontId::proportional(theme::POLICE_CORPS),
                    p.fg_muted.gamma_multiply(proximite),
                );
                job.wrap.max_width = largeur_texte;
                job.wrap.max_rows = 1;
                job.wrap.break_anywhere = true;
                job.wrap.overflow_character = Some('…');
                let galley = ui.painter().layout_job(job);
                peintre.galley(egui::pos2(x, y + 5.0), galley, p.fg_muted);
            }
        }
    }

    /// Bannière du jeu focalisé, en fond d'écran.
    fn dessiner_banniere(&mut self, ui: &egui::Ui, peintre: &egui::Painter, zone: egui::Rect, e: Option<&Element>) {
        let Some(Visuel::Jeu(id)) = e.map(|e| e.visuel) else {
            return;
        };
        let fichier = self.bib.jeu(id).and_then(|j| j.banniere.clone());
        let texture = fichier.and_then(|f| self.icones.visuel(id, Genre::Banniere, &f));
        if let Some(t) = texture {
            let uv = icons::uv_couvrir(t.size_vec2(), zone.size());
            egui::Image::new(&t).uv(uv).paint_at(ui, zone);
            peintre.rect_filled(zone, CornerRadius::ZERO, voile(theme::p().bg_main, 215));
        }
    }

    fn dessiner_jaquette_carrousel(
        &mut self,
        ui: &egui::Ui,
        peintre: &egui::Painter,
        id: u64,
        cadre: egui::Rect,
        distance: f32,
    ) {
        let p = theme::p();
        let Some(jeu) = self.bib.jeu(id).cloned() else {
            return;
        };
        let manquant = self.bib.manquants.contains(&id);
        // La haute définition n'est demandée qu'à l'arrêt, sinon un défilement
        // rapide lancerait le décodage de toute la bibliothèque.
        let a_l_arret = distance.abs() < 0.02;
        let fichier = jeu.jaquette.as_deref();
        let grande = fichier.filter(|_| a_l_arret).and_then(|f| self.icones.grande_jaquette(id, f));
        let texture = grande.or_else(|| fichier.and_then(|f| self.icones.visuel(id, Genre::Jaquette, f)));
        self.dessiner_couverture(ui, &jeu, cadre, manquant, texture);

        let proximite = (1.0 - distance.abs()).clamp(0.0, 1.0);
        let opacite = (150.0 * (1.0 - proximite)) as u8;
        if opacite > 0 {
            peintre.rect_filled(cadre, CornerRadius::same(theme::RAYON_M), voile(p.bg_main, opacite));
        }
        marquer_statut(peintre, cadre.left_top() + Vec2::splat(theme::ESPACE_L), jeu.statut);
        if jeu.favori {
            peintre.text(
                cadre.right_top() + Vec2::new(-theme::ESPACE_S, theme::ESPACE_S),
                Align2::RIGHT_TOP,
                "⭐",
                FontId::proportional(theme::POLICE_XMB),
                p.accent,
            );
        }
        if proximite > 0.05 {
            peintre.rect_stroke(
                cadre,
                CornerRadius::same(theme::RAYON_M),
                Stroke::new(3.0 * proximite, p.accent.gamma_multiply(proximite)),
                egui::StrokeKind::Outside,
            );
        }
    }

    /// Jaquettes en ligne : celle du jeu focalisé au centre, en grand.
    fn dessiner_carrousel(&mut self, ui: &egui::Ui, zone: egui::Rect, elements: &[Element], focus: usize) {
        let p = theme::p();
        let pos = self.xmb.pos_liste.pos;
        let peintre = ui.painter_at(zone);
        // La marge du haut laisse la place au contour de la jaquette.
        let hauteur = (zone.height() - HAUTEUR_LEGENDE - theme::ESPACE_M).max(HAUTEUR_JAQUETTE_MIN);
        let largeur = hauteur / RATIO_JAQUETTE;
        let centre = egui::pos2(zone.center().x, zone.top() + theme::ESPACE_M + hauteur / 2.0);

        // Seules les jaquettes à l'écran sont dessinées, les plus proches en dernier.
        let visibles = (zone.width() / 2.0 / (largeur * ECHELLE_VOISINE + ECART_JAQUETTES)).ceil() as isize + 2;
        let debut = (pos.round() as isize - visibles).max(0) as usize;
        let fin = ((pos.round() as isize + visibles + 1).max(0) as usize).min(elements.len());
        let mut ordre: Vec<usize> = (debut..fin).collect();
        ordre.sort_by(|a, b| {
            let d = |i: usize| (i as f32 - pos).abs();
            d(*b).total_cmp(&d(*a))
        });
        for i in ordre {
            let Visuel::Jeu(id) = elements[i].visuel else {
                continue;
            };
            let distance = i as f32 - pos;
            let (decalage, echelle) = place_carrousel(distance, largeur);
            let cadre = egui::Rect::from_center_size(
                centre + Vec2::new(decalage, 0.0),
                Vec2::new(largeur, hauteur) * echelle,
            );
            self.dessiner_jaquette_carrousel(ui, &peintre, id, cadre, distance);
        }

        let Some(Element { titre, visuel: Visuel::Jeu(id), .. }) = elements.get(focus) else {
            return;
        };
        let bas = centre.y + hauteur / 2.0 + theme::ESPACE_L;
        peintre.text(
            egui::pos2(centre.x, bas),
            Align2::CENTER_TOP,
            titre,
            FontId::new(theme::POLICE_TITRE, theme::gras()),
            p.fg,
        );
        if let Some(jeu) = self.bib.jeu(*id) {
            peintre.text(
                egui::pos2(centre.x, bas + 40.0),
                Align2::CENTER_TOP,
                resume_jeu(jeu),
                FontId::proportional(theme::POLICE_CORPS),
                p.fg_muted,
            );
        }
    }

    fn dessiner_aide(&self, peintre: &egui::Painter, zone: egui::Rect, e: Option<&Element>) {
        let p = theme::p();
        let famille = self.famille_glyphes();
        let mut aides: Vec<(Bouton, &str)> = Vec::new();
        if self.xmb.clavier.is_some() {
            aides.extend([
                (Bouton::A, "Taper"),
                (Bouton::X, "Effacer"),
                (Bouton::Y, "Espace"),
                (Bouton::Gachettes, "Majuscule"),
                (Bouton::Start, "Valider"),
                (Bouton::B, "Annuler"),
            ]);
        } else if self.scan.is_some() || self.lot_en_cours() {
            aides.push((Bouton::B, "Annuler"));
        } else {
            let est_jeu = matches!(e.map(|e| e.visuel), Some(Visuel::Jeu(_)));
            let actif = e.is_some_and(|e| !matches!(e.effet, Effet::Aucun));
            if actif {
                aides.push((Bouton::A, if est_jeu { "Lancer" } else { "Valider" }));
            }
            if est_jeu {
                aides.push((Bouton::Y, "Options"));
            }
            aides.push((Bouton::X, "Recherche"));
            if !self.xmb.pile.is_empty() {
                aides.push((Bouton::B, "Retour"));
            }
            aides.push((Bouton::Gachettes, "Catégories"));
            if est_jeu {
                aides.push((Bouton::Direction, "Défiler"));
            }
            aides.push((Bouton::Start, "Paramètres"));
        }

        let y = zone.bottom() - HAUTEUR_BARRE_AIDE / 2.0;
        let mut x = zone.left() + theme::ESPACE_XXL + 13.0;
        for (b, texte) in aides {
            let largeur = glyphe(peintre, egui::pos2(x, y), b, famille);
            let t = peintre.text(
                egui::pos2(x + largeur - 13.0 + theme::ESPACE_S, y),
                Align2::LEFT_CENTER,
                texte,
                FontId::proportional(theme::POLICE_CORPS),
                p.fg,
            );
            x = t.right() + theme::ESPACE_XL + 13.0;
        }
    }

    fn dessiner_entete(&self, peintre: &egui::Painter, zone: egui::Rect) {
        let p = theme::p();
        let droite = zone.right() - theme::ESPACE_XXL;
        let heure = peintre.text(
            egui::pos2(droite, zone.top() + theme::ESPACE_XL),
            Align2::RIGHT_TOP,
            chrono::Local::now().format("%H:%M").to_string(),
            FontId::new(theme::POLICE_TITRE, theme::gras()),
            p.fg,
        );
        let manette = match &self.manette_connectee {
            Some((nom, _)) => format!("🎮  {nom}"),
            None => "⌨  Clavier".to_string(),
        };
        peintre.text(
            egui::pos2(heure.left() - theme::ESPACE_L, heure.center().y),
            Align2::RIGHT_CENTER,
            manette,
            FontId::proportional(theme::POLICE_LEGENDE),
            p.fg_muted,
        );
    }

    pub(super) fn vue_xmb(&mut self, ui: &mut egui::Ui) {
        let p = theme::p();
        let ctx = ui.ctx().clone();
        self.synchroniser_xmb();
        let mut elements = self.elements_courants();
        if self.appliquer_focus_voulu(&elements) {
            elements = self.elements_courants();
            self.appliquer_focus_voulu(&elements);
        }
        let focus = self.xmb.focus_courant().min(elements.len().saturating_sub(1));
        self.xmb.definir_focus(focus);

        let anime = self.bib.config.fond_anime;
        let actif = ctx.input(|i| i.viewport().focused != Some(false));
        if anime && actif {
            ctx.request_repaint_after(INTERVALLE_VAGUES);
        } else {
            ctx.request_repaint_after(Duration::from_secs(20)); // L'horloge.
        }
        let temps = if anime { ctx.input(|i| i.time) as f32 } else { 0.0 };

        let cle = (self.xmb.categorie, self.xmb.pile.len());
        if self.xmb.cle_liste != cle {
            self.xmb.cle_liste = cle;
            self.xmb.pos_liste = Lisseur::a(focus as f32);
        }
        let dt = ctx.input(|i| i.stable_dt).min(0.05);
        let cible_categorie = self.xmb.categorie as f32;
        self.xmb.pos_liste.avancer(focus as f32, dt);
        self.xmb.pos_categorie.avancer(cible_categorie, dt);
        if self.xmb.pos_liste.pos != focus as f32 || self.xmb.pos_categorie.pos != cible_categorie {
            ctx.request_repaint(); // Le défilement passe avant le plafond du fond animé.
        }
        let carrousel = est_carrousel(&elements);

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg_main))
            .show(ui, |ui| {
                let zone = ui.max_rect();
                let peintre = ui.painter_at(zone);
                let courant = elements.get(focus);
                self.dessiner_banniere(ui, &peintre, zone, courant);
                fond::peindre(&peintre, zone, p.accent, temps);

                self.dessiner_categories(&peintre, zone, self.xmb.pos_categorie.pos);
                self.dessiner_entete(&peintre, zone);

                let mut haut = zone.top() + 170.0;
                if let Some(page) = self.xmb.pile.last().map(|n| n.page.clone()) {
                    peintre.text(
                        egui::pos2(zone.center().x - 180.0, haut),
                        Align2::LEFT_TOP,
                        self.titre_page_xmb(&page),
                        FontId::new(theme::POLICE_SECTION, theme::gras()),
                        p.accent,
                    );
                    haut += theme::ESPACE_XXL;
                }
                let liste = egui::Rect::from_min_max(
                    egui::pos2(zone.left(), haut),
                    egui::pos2(zone.right(), zone.bottom() - HAUTEUR_BARRE_AIDE),
                );
                if carrousel {
                    self.dessiner_carrousel(ui, liste, &elements, focus);
                } else {
                    self.dessiner_liste(ui, liste, &elements, focus);
                }

                self.dessiner_aide(&peintre, zone, courant);
                if let Some(clavier) = &self.xmb.clavier {
                    clavier.dessiner(ui, zone);
                    self.dessiner_aide(&ui.painter_at(zone), zone, courant);
                }
            });
    }
}

/// Nom d'export qui n'écrase aucun fichier du dossier.
fn chemin_export_libre(dossier: &Path, date: &str) -> PathBuf {
    let premier = dossier.join(format!("omni_launcher_{date}.json"));
    if !premier.exists() {
        return premier;
    }
    (2..)
        .map(|i| dossier.join(format!("omni_launcher_{date}_{i}.json")))
        .find(|p| !p.exists())
        .unwrap_or(premier)
}

/// Coche le premier exécutable de chaque jeu, hors doublons.
pub(super) fn cocher_principaux(candidats: &mut [Candidat]) {
    let mut vus = HashSet::new();
    for c in candidats {
        let principal = vus.insert(c.groupe.clone()) || c.groupe == "(fichiers isolés)";
        c.coche = principal && !c.doublon;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(visuel: Visuel) -> Element {
        Element {
            titre: String::new(),
            detail: String::new(),
            visuel,
            effet: Effet::Aucun,
        }
    }

    #[test]
    fn le_lissage_garde_une_vitesse_reguliere_sous_une_cible_qui_avance() {
        // Une répétition toutes les 110 ms, affichée à 60 images par seconde.
        let dt = 1.0 / 60.0;
        let (mut l, mut cible, mut vitesses) = (Lisseur::default(), 0.0, Vec::new());
        for image in 0..240 {
            if image % 7 == 0 {
                cible += 1.0;
            }
            let avant = l.pos;
            l.avancer(cible, dt);
            assert!(l.pos >= avant, "jamais de recul");
            vitesses.push(l.pos - avant);
        }
        // Une fois lancé, le défilement ne s'arrête jamais entre deux répétitions.
        let regime = &vitesses[60..];
        let lente = regime.iter().cloned().fold(f32::INFINITY, f32::min);
        let rapide = regime.iter().cloned().fold(0.0, f32::max);
        assert!(lente > rapide * 0.5, "{lente} contre {rapide}");
    }

    #[test]
    fn le_lissage_arrive_exactement_et_sans_depasser() {
        let (mut l, cible) = (Lisseur::default(), 3.0);
        let mut images = 0;
        while l.pos != cible {
            l.avancer(cible, 1.0 / 60.0);
            assert!(l.pos <= cible);
            images += 1;
            assert!(images < 120, "n'arrive jamais");
        }
    }

    #[test]
    fn le_lissage_reste_stable_sur_une_image_lente() {
        let mut l = Lisseur::a(0.0);
        l.avancer(5.0, 0.05);
        assert!(l.pos > 0.0 && l.pos < 5.0, "{}", l.pos);
    }

    #[test]
    fn seules_les_listes_de_jeux_sont_en_carrousel() {
        assert!(est_carrousel(&[element(Visuel::Jeu(1)), element(Visuel::Jeu(2))]));
        assert!(!est_carrousel(&[element(Visuel::Jeu(1)), element(Visuel::Glyphe("x"))]));
        assert!(!est_carrousel(&[]));
    }

    #[test]
    fn le_carrousel_garde_l_ordre_sans_chevauchement() {
        let largeur = 400.0;
        let (centre, echelle) = place_carrousel(0.0, largeur);
        assert_eq!((centre, echelle), (0.0, 1.0));
        let (droite, _) = place_carrousel(1.0, largeur);
        let (gauche, _) = place_carrousel(-1.0, largeur);
        assert_eq!(gauche, -droite);
        assert!(droite >= largeur * (1.0 + ECHELLE_VOISINE) / 2.0);

        // Le mouvement reste continu et monotone entre deux positions.
        let mut precedent = f32::NEG_INFINITY;
        for i in -300..=300 {
            let (x, _) = place_carrousel(i as f32 / 100.0, largeur);
            assert!(x > precedent);
            precedent = x;
        }
    }

    #[test]
    fn le_focus_reste_dans_la_liste() {
        assert_eq!(deplacer_focus(0, 5, -1), 0);
        assert_eq!(deplacer_focus(0, 5, 1), 1);
        assert_eq!(deplacer_focus(4, 5, 1), 4);
        assert_eq!(deplacer_focus(3, 0, 1), 0);
    }

    #[test]
    fn les_categories_bouclent_et_ferment_les_pages() {
        let mut x = EtatXmb::default();
        x.ouvrir(Page::Tri);
        x.changer_categorie(-1);
        assert_eq!(x.categorie, CATEGORIES.len() - 1);
        assert!(x.pile.is_empty());
        x.changer_categorie(1);
        assert_eq!(x.categorie, 0);
    }

    #[test]
    fn chaque_categorie_garde_son_focus() {
        let mut x = EtatXmb::default();
        x.definir_focus(3);
        x.changer_categorie(1);
        assert_eq!(x.focus_courant(), 0);
        x.changer_categorie(-1);
        assert_eq!(x.focus_courant(), 3);
    }

    #[test]
    fn une_sous_page_a_son_propre_focus() {
        let mut x = EtatXmb::default();
        x.definir_focus(2);
        x.ouvrir(Page::Tri);
        x.definir_focus(1);
        assert_eq!(x.focus_courant(), 1);
        x.pile.pop();
        assert_eq!(x.focus_courant(), 2);
    }

    #[test]
    fn une_page_s_ouvre_par_dessus_une_confirmation() {
        let mut x = EtatXmb::default();
        x.synchroniser(true, false);
        assert_eq!(x.pile.len(), 1);
        x.ouvrir(Page::Fichiers(But::Localiser(1), None));
        x.synchroniser(true, false);
        assert_eq!(x.pile.len(), 2, "la confirmation ne repasse pas devant");
        assert!(matches!(x.pile[1].page, Page::Fichiers(..)));
        x.synchroniser(false, false);
        assert_eq!(x.pile.len(), 1, "seule la confirmation disparaît");
        assert!(matches!(x.pile[0].page, Page::Fichiers(..)));
    }

    #[test]
    fn revue_et_confirmation_bloquent_le_changement_de_categorie() {
        let mut x = EtatXmb::default();
        assert!(!x.page_bloquante());
        x.synchroniser(false, true);
        assert!(x.page_bloquante());
        x.synchroniser(false, false);
        assert!(!x.page_bloquante());
    }

    #[test]
    fn l_export_n_ecrase_pas_un_fichier_existant() {
        let d = std::env::temp_dir().join(format!("omni_export_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let premier = chemin_export_libre(&d, "2026-11-06");
        assert!(premier.ends_with("omni_launcher_2026-11-06.json"));
        std::fs::write(&premier, "{}").unwrap();
        let second = chemin_export_libre(&d, "2026-11-06");
        assert!(second.ends_with("omni_launcher_2026-11-06_2.json"));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn lecture_d_une_couleur() {
        assert_eq!(analyser_couleur("ff8800"), Some([255, 136, 0]));
        assert_eq!(analyser_couleur(" #FF8800 "), Some([255, 136, 0]));
        assert_eq!(analyser_couleur("ff88"), None);
        assert_eq!(analyser_couleur("gggggg"), None);
        assert_eq!(analyser_couleur("éééé"), None);
    }

    #[test]
    fn principaux_sans_doublons() {
        let candidat = |groupe: &str, doublon| Candidat {
            exe: PathBuf::from("C:/a.exe"),
            nom: "A".into(),
            cible_suppression: PathBuf::from("C:/"),
            coche: false,
            groupe: groupe.into(),
            doublon,
        };
        let mut c = vec![candidat("A", false), candidat("A", false), candidat("B", true)];
        cocher_principaux(&mut c);
        assert_eq!(c.iter().map(|x| x.coche).collect::<Vec<_>>(), [true, false, false]);
    }
}
