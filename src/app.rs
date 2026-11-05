//! Interface principale : bibliothèque, recherche, scan, dialogues.
//! Les vues Paramètres, Statistiques et le mode plein écran sont dans les
//! sous-modules.

mod liste;
mod mises_a_jour;
mod parametres;
mod statistiques;
mod tv;
mod visuels;

use crate::config::{
    Affichage, Config, Dossiers, Jeu, Statut, TAILLE_TUILE_MAX, TAILLE_TUILE_MIN, Tri, cle_chemin,
    maintenant,
};
use crate::icons::{self, ChargeurIcones, Genre};
use crate::manette::{self, Commande, MsgManette};
use crate::partie::{self, MsgPartie};
use crate::sauvegarde::{self, InfoSauvegarde};
use crate::scan::{self, Candidat};
use crate::stats;
use crate::steamgriddb;
use crate::theme;
use crate::platform;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Key, RichText, Sense, Stroke, Vec2,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

const DUREE_NOTIF: Duration = Duration::from_secs(4);
/// Délai entre le lancement d'un jeu et la réduction du launcher, le temps
/// de lire la notification.
const DELAI_REDUCTION: Duration = Duration::from_millis(1200);
/// Proportions d'une jaquette : 600x900.
const RATIO_JAQUETTE: f32 = 1.5;
/// Longueur de l'aperçu des notes dans les infobulles.
const APERCU_NOTES_MAX: usize = 90;

/// Action différée exécutée après la fermeture d'une boîte de dialogue.
type ActionDialogue = Box<dyn FnOnce(&mut App, &egui::Context)>;

#[derive(PartialEq, Eq, Clone, Copy)]
enum Vue {
    Bibliotheque,
    Parametres,
    Statistiques,
}

enum Action {
    Lancer(u64),
    BasculerFavori(u64),
    DebuterRenommage(u64),
    Renommer(u64, String),
    ChangerIcone(u64),
    RetablirIcone(u64),
    OuvrirEmplacement(u64),
    EditerArguments(u64),
    EditerTags(u64),
    EditerNotes(u64),
    EditerVisuels(u64),
    DemanderRetrait(u64),
    DemanderSuppression(u64),
}

enum Dialogue {
    Retirer(u64),
    Supprimer { id: u64, cible: PathBuf },
    SuppressionRefusee { id: u64, raison: String },
    Introuvable(u64),
    RetirerIntrouvables,
    Arguments { id: u64, texte: String },
    Tags { id: u64, saisie: String },
    Notes { id: u64, statut: Statut, texte: String },
    RenommerTag { ancien: String, texte: String },
    SupprimerTag(String),
    Restaurer(InfoSauvegarde),
    Importer { config: Box<Config>, fichier: String },
}

enum MsgScan {
    Progression {
        fait: usize,
        total: usize,
        nom: String,
    },
    Fini(Vec<Candidat>),
}

struct ScanEnCours {
    reception: Receiver<MsgScan>,
    annule: Arc<AtomicBool>,
    fait: usize,
    total: usize,
    nom: String,
}

struct Revue {
    candidats: Vec<Candidat>,
}

struct Renommage {
    id: u64,
    texte: String,
    premiere_frame: bool,
}

struct Notification {
    texte: String,
    erreur: bool,
    depuis: Instant,
}

/// Jeu lancé dont on mesure le temps de jeu.
struct PartieEnCours {
    id: u64,
    nom: String,
    /// Horodatage de détection du jeu, une fois qu'il tourne.
    debut: Option<u64>,
    reception: Receiver<MsgPartie>,
}

pub struct App {
    dossiers: Dossiers,
    config: Config,
    icones: ChargeurIcones,
    vue: Vue,
    recherche: String,
    focus_recherche: bool,
    filtre_tag: Option<String>,
    filtre_statut: Option<Statut>,
    renommage: Option<Renommage>,
    dialogue: Option<Dialogue>,
    scan: Option<ScanEnCours>,
    revue: Option<Revue>,
    manquants: HashSet<u64>,
    notification: Option<Notification>,
    suppressions: Vec<Receiver<Result<String, String>>>,
    parties: Vec<PartieEnCours>,
    reduire_a: Option<Instant>,
    /// Une modification attend la fin d'un glisser pour être enregistrée.
    sauvegarde_differee: bool,
    sauvegardes: Vec<InfoSauvegarde>,
    manette: Receiver<MsgManette>,
    manette_connectee: Option<String>,
    commandes: Vec<Commande>,
    mode_tv: bool,
    tv: tv::EtatTv,
    cle_sgdb: String,
    edition_visuels: Option<visuels::EditionVisuels>,
    lot_visuels: Option<visuels::LotVisuels>,
    maj: mises_a_jour::EtatMaj,
}

impl App {
    pub fn nouveau(ctx: &egui::Context) -> Self {
        let dossiers = Dossiers::detecter();
        let config = Config::charger(&dossiers.config);
        theme::appliquer(ctx, config.theme, config.accent_perso);
        // Copie de la configuration telle qu'elle était au premier lancement du jour.
        let _ = sauvegarde::sauvegarde_du_jour(&dossiers);
        // La taille de fenêtre mémorisée ne doit pas rouvrir en plein écran.
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));

        let icones = ChargeurIcones::nouveau(ctx, &dossiers);
        let cle_sgdb = steamgriddb::lire_cle(&dossiers);
        let mut app = Self {
            dossiers,
            config,
            icones,
            vue: Vue::Bibliotheque,
            recherche: String::new(),
            focus_recherche: false,
            filtre_tag: None,
            filtre_statut: None,
            renommage: None,
            dialogue: None,
            scan: None,
            revue: None,
            manquants: HashSet::new(),
            notification: None,
            suppressions: Vec::new(),
            parties: Vec::new(),
            reduire_a: None,
            sauvegarde_differee: false,
            sauvegardes: Vec::new(),
            manette: manette::demarrer(ctx.clone()),
            manette_connectee: None,
            commandes: Vec::new(),
            mode_tv: false,
            tv: tv::EtatTv::default(),
            cle_sgdb,
            edition_visuels: None,
            lot_visuels: None,
            maj: mises_a_jour::EtatMaj::Inconnu,
        };
        app.verifier_manquants();
        if app.config.verifier_maj {
            app.chercher_maj(ctx, false);
        }
        app
    }

    // ------------------------------------------------------------------
    // Utilitaires
    // ------------------------------------------------------------------

    fn sauver(&mut self) {
        self.sauvegarde_differee = false;
        if let Err(e) = self.config.sauvegarder(&self.dossiers.config) {
            self.notifier_erreur(format!("Impossible d'enregistrer la configuration : {e}"));
        }
    }

    fn notifier(&mut self, texte: impl Into<String>) {
        self.notification = Some(Notification {
            texte: texte.into(),
            erreur: false,
            depuis: Instant::now(),
        });
    }

    fn notifier_erreur(&mut self, texte: impl Into<String>) {
        self.notification = Some(Notification {
            texte: texte.into(),
            erreur: true,
            depuis: Instant::now(),
        });
    }

    fn verifier_manquants(&mut self) {
        self.manquants = self
            .config
            .jeux
            .iter()
            .filter(|j| !j.exe.is_file())
            .map(|j| j.id)
            .collect();
    }

    fn jeu(&self, id: u64) -> Option<&Jeu> {
        self.config.jeux.iter().find(|j| j.id == id)
    }

    fn appliquer_theme(&self, ctx: &egui::Context) {
        theme::appliquer(ctx, self.config.theme, self.config.accent_perso);
    }

    /// Jeux filtrés (recherche sur le nom et les tags, filtres de tag et de
    /// statut) et triés, séparés en (favoris, autres).
    fn trier_filtrer(
        &self,
        recherche: &str,
        tag: Option<&str>,
        statut: Option<Statut>,
    ) -> (Vec<u64>, Vec<u64>) {
        let termes: Vec<String> = recherche
            .split_whitespace()
            .map(|t| t.to_lowercase())
            .collect();

        let mut jeux: Vec<&Jeu> = self
            .config
            .jeux
            .iter()
            .filter(|j| tag.is_none_or(|t| j.a_tag(t)))
            .filter(|j| statut.is_none_or(|s| j.statut == s))
            .filter(|j| {
                let nom = j.nom.to_lowercase();
                termes.iter().all(|t| {
                    nom.contains(t.as_str())
                        || j.tags.iter().any(|g| g.to_lowercase().contains(t.as_str()))
                })
            })
            .collect();

        match self.config.tri {
            Tri::Nom => jeux.sort_by_cached_key(|j| j.nom.to_lowercase()),
            Tri::DernierLancement => jeux.sort_by(|a, b| {
                b.dernier_lancement
                    .cmp(&a.dernier_lancement)
                    .then_with(|| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()))
            }),
            Tri::DateAjout => {
                jeux.sort_by(|a, b| b.ajoute_le.cmp(&a.ajoute_le).then(b.id.cmp(&a.id)))
            }
            Tri::TempsDeJeu => jeux.sort_by(|a, b| {
                b.temps_jeu
                    .cmp(&a.temps_jeu)
                    .then_with(|| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()))
            }),
        }

        let (fav, autres): (Vec<&Jeu>, Vec<&Jeu>) = jeux.into_iter().partition(|j| j.favori);
        (
            fav.iter().map(|j| j.id).collect(),
            autres.iter().map(|j| j.id).collect(),
        )
    }

    fn listes_visibles(&self) -> (Vec<u64>, Vec<u64>) {
        self.trier_filtrer(&self.recherche, self.filtre_tag.as_deref(), self.filtre_statut)
    }

    // ------------------------------------------------------------------
    // Ajout et scan
    // ------------------------------------------------------------------

    fn ajouter_exes(&mut self, exes: Vec<PathBuf>) {
        let mut ajoutes = 0;
        let mut dernier_nom = String::new();
        for exe in exes {
            if self.config.contient_exe(&exe) {
                continue;
            }
            let stem = exe
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let nom = scan::embellir_nom(&stem);
            let id = self.config.nouvel_id();
            dernier_nom = nom.clone();
            self.config.jeux.push(Jeu::nouveau(id, nom, exe, None));
            ajoutes += 1;
        }
        match ajoutes {
            0 => self.notifier("Ce jeu est déjà dans la bibliothèque."),
            1 => {
                self.sauver();
                self.notifier(format!(
                    "« {dernier_nom} » ajouté. Clic droit pour le renommer."
                ));
            }
            n => {
                self.sauver();
                self.notifier(format!("{n} jeux ajoutés."));
            }
        }
        self.verifier_manquants();
    }

    fn choisir_exe(&mut self) {
        if let Some(fichiers) = rfd::FileDialog::new()
            .set_title("Sélectionnez le ou les exécutables")
            .add_filter("Exécutables", &["exe"])
            .pick_files()
        {
            self.ajouter_exes(fichiers);
        }
    }

    fn choisir_dossier_scan(&mut self, ctx: &egui::Context) {
        let mut dialogue = rfd::FileDialog::new()
            .set_title("Dossier contenant vos jeux (un sous-dossier par jeu)");
        if let Some(d) = &self.config.dernier_dossier_scan {
            dialogue = dialogue.set_directory(d);
        }
        if let Some(dossier) = dialogue.pick_folder() {
            self.lancer_scan(dossier, ctx);
        }
    }

    fn lancer_scan(&mut self, dossier: PathBuf, ctx: &egui::Context) {
        if self.scan.is_some() {
            return;
        }
        self.config.dernier_dossier_scan = Some(dossier.clone());
        self.sauver();

        let existants: HashSet<String> = self
            .config
            .jeux
            .iter()
            .map(|j| cle_chemin(&j.exe))
            .collect();
        let (tx, rx) = channel();
        let annule = Arc::new(AtomicBool::new(false));
        let annule_thread = Arc::clone(&annule);
        let ctx = ctx.clone();

        std::thread::spawn(move || {
            let tx_prog = tx.clone();
            let ctx_prog = ctx.clone();
            let resultat = scan::scanner(
                &dossier,
                |p| existants.contains(&cle_chemin(p)),
                |fait, total, nom| {
                    let _ = tx_prog.send(MsgScan::Progression {
                        fait,
                        total,
                        nom: nom.to_string(),
                    });
                    ctx_prog.request_repaint();
                },
                &annule_thread,
            );
            let _ = tx.send(MsgScan::Fini(resultat));
            ctx.request_repaint();
        });

        self.scan = Some(ScanEnCours {
            reception: rx,
            annule,
            fait: 0,
            total: 0,
            nom: String::new(),
        });
    }

    fn suivre_scan(&mut self) {
        let Some(scan) = &mut self.scan else { return };
        let mut fini = None;
        while let Ok(msg) = scan.reception.try_recv() {
            match msg {
                MsgScan::Progression { fait, total, nom } => {
                    scan.fait = fait;
                    scan.total = total;
                    scan.nom = nom;
                }
                MsgScan::Fini(c) => fini = Some(c),
            }
        }
        if let Some(candidats) = fini {
            let annule = scan.annule.load(Ordering::Relaxed);
            self.scan = None;
            if annule {
                self.notifier("Scan annulé.");
            } else if candidats.is_empty() {
                self.notifier("Aucun nouveau jeu trouvé dans ce dossier.");
            } else {
                self.revue = Some(Revue { candidats });
            }
        }
    }

    fn importer_revue(&mut self, candidats: Vec<Candidat>) {
        let mut n = 0;
        for c in candidats.into_iter().filter(|c| c.coche) {
            if self.config.contient_exe(&c.exe) {
                continue;
            }
            let id = self.config.nouvel_id();
            let nom = if c.nom.trim().is_empty() {
                c.groupe.clone()
            } else {
                c.nom.trim().to_string()
            };
            self.config
                .jeux
                .push(Jeu::nouveau(id, nom, c.exe, Some(c.cible_suppression)));
            n += 1;
        }
        self.sauver();
        self.verifier_manquants();
        self.notifier(match n {
            0 => "Aucun jeu importé.".to_string(),
            1 => "1 jeu importé.".to_string(),
            n => format!("{n} jeux importés."),
        });
    }

    // ------------------------------------------------------------------
    // Actions sur les jeux
    // ------------------------------------------------------------------

    fn executer(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::Lancer(id) => self.lancer(id, ctx),
            Action::BasculerFavori(id) => {
                if let Some(j) = self.config.jeu_mut(id) {
                    j.favori = !j.favori;
                }
                self.sauver();
            }
            Action::DebuterRenommage(id) => {
                if let Some(j) = self.jeu(id) {
                    self.renommage = Some(Renommage {
                        id,
                        texte: j.nom.clone(),
                        premiere_frame: true,
                    });
                }
            }
            Action::Renommer(id, nom) => {
                let nom = nom.trim().to_string();
                if !nom.is_empty() {
                    if let Some(j) = self.config.jeu_mut(id) {
                        j.nom = nom;
                    }
                    self.sauver();
                }
            }
            Action::ChangerIcone(id) => self.changer_icone(id),
            Action::RetablirIcone(id) => {
                let ancien = self.config.jeu_mut(id).and_then(|j| j.icone_perso.take());
                if let Some(nom) = ancien {
                    let _ = std::fs::remove_file(self.dossiers.icones.join(nom));
                }
                self.icones.invalider(id);
                self.sauver();
            }
            Action::OuvrirEmplacement(id) => {
                if let Some(exe) = self.jeu(id).map(|j| j.exe.clone())
                    && let Err(e) = platform::ouvrir_dans_explorateur(&exe)
                {
                    self.notifier_erreur(format!("Impossible d'ouvrir l'emplacement : {e}"));
                }
            }
            Action::EditerArguments(id) => {
                if let Some(j) = self.jeu(id) {
                    self.dialogue = Some(Dialogue::Arguments {
                        id,
                        texte: j.arguments.clone(),
                    });
                }
            }
            Action::EditerTags(id) => {
                self.dialogue = Some(Dialogue::Tags {
                    id,
                    saisie: String::new(),
                });
            }
            Action::EditerNotes(id) => {
                if let Some(j) = self.jeu(id) {
                    self.dialogue = Some(Dialogue::Notes {
                        id,
                        statut: j.statut,
                        texte: j.notes.clone(),
                    });
                }
            }
            Action::EditerVisuels(id) => self.ouvrir_visuels(id),
            Action::DemanderRetrait(id) => self.dialogue = Some(Dialogue::Retirer(id)),
            Action::DemanderSuppression(id) => {
                let Some(jeu) = self.jeu(id) else { return };
                self.dialogue = Some(match self.verifier_suppression(jeu) {
                    Ok(cible) => Dialogue::Supprimer { id, cible },
                    Err(raison) => Dialogue::SuppressionRefusee { id, raison },
                });
            }
        }
    }

    fn lancer(&mut self, id: u64, ctx: &egui::Context) {
        let Some(jeu) = self.jeu(id).cloned() else {
            return;
        };
        if !jeu.exe.is_file() {
            self.manquants.insert(id);
            self.dialogue = Some(Dialogue::Introuvable(id));
            return;
        }
        let suivi_existant = self.parties.iter().any(|p| p.id == id);
        let mut cible = partie::Cible::nouvelle(&jeu.exe, jeu.dossier_jeu().as_deref());
        if !suivi_existant {
            cible.ignorer_existants(&platform::processus_actifs());
        }
        match platform::lancer(&jeu.exe, &jeu.arguments) {
            Ok(()) => {
                if let Some(j) = self.config.jeu_mut(id) {
                    j.dernier_lancement = Some(maintenant());
                    j.nb_lancements += 1;
                }
                self.sauver();
                self.notifier(format!("Lancement de « {} »…", jeu.nom));

                if !suivi_existant {
                    self.parties.push(PartieEnCours {
                        id,
                        nom: jeu.nom.clone(),
                        debut: None,
                        reception: partie::surveiller(cible, ctx.clone()),
                    });
                }
                // Le launcher s'efface pendant la partie et revient à la fin.
                self.reduire_a = Some(Instant::now() + DELAI_REDUCTION);
                ctx.request_repaint_after(DELAI_REDUCTION);
            }
            Err(e) => self.notifier_erreur(format!("Impossible de lancer « {} » : {e}", jeu.nom)),
        }
    }

    /// Relève les messages des parties en cours. Appelé même quand la
    /// fenêtre est réduite (voir [`eframe::App::logic`]).
    fn suivre_parties(&mut self, ctx: &egui::Context) {
        if self.parties.is_empty() {
            return;
        }
        let mut terminees = Vec::new();
        let mut messages = Vec::new();
        for (i, p) in self.parties.iter_mut().enumerate() {
            while let Ok(m) = p.reception.try_recv() {
                match m {
                    MsgPartie::Debut(t) => p.debut = Some(t),
                    MsgPartie::Fin { debut, fin } => {
                        terminees.push(i);
                        messages.push((p.id, p.nom.clone(), Some((debut, fin))));
                    }
                    MsgPartie::NonDetectee => {
                        terminees.push(i);
                        messages.push((p.id, p.nom.clone(), None));
                    }
                }
            }
        }
        if terminees.is_empty() {
            return;
        }
        for i in terminees.into_iter().rev() {
            self.parties.remove(i);
        }

        for (id, nom, mesure) in messages {
            match mesure {
                Some((debut, fin)) => {
                    let duree = fin.saturating_sub(debut);
                    if let Some(j) = self.config.jeu_mut(id) {
                        j.ajouter_session(debut, duree);
                    }
                    self.sauver();
                    self.notifier(format!(
                        "Partie de « {nom} » terminée : {}.",
                        stats::duree(duree)
                    ));
                }
                None => self.notifier(format!(
                    "Le temps de jeu de « {nom} » n'a pas pu être mesuré (jeu non détecté)."
                )),
            }
        }

        if self.parties.is_empty() {
            self.reduire_a = None;
            if self.config.fermer_apres_lancement {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }
    }

    /// Enregistre le temps des parties encore en cours (fermeture du launcher).
    fn enregistrer_parties_en_cours(&mut self) {
        let fin = maintenant();
        let mut modifie = false;
        for p in std::mem::take(&mut self.parties) {
            if let Some(debut) = p.debut
                && let Some(j) = self.config.jeu_mut(p.id)
            {
                j.ajouter_session(debut, fin.saturating_sub(debut));
                modifie = true;
            }
        }
        if modifie {
            let _ = self.config.sauvegarder(&self.dossiers.config);
        }
    }

    fn changer_icone(&mut self, id: u64) {
        let Some(nom_jeu) = self.jeu(id).map(|j| j.nom.clone()) else {
            return;
        };
        let Some(fichier) = rfd::FileDialog::new()
            .set_title(format!("Nouvelle icône pour {nom_jeu}"))
            .add_filter("Images", &["png", "jpg", "jpeg", "ico", "bmp", "webp"])
            .pick_file()
        else {
            return;
        };
        match icons::importer_icone(&fichier, id, &self.dossiers) {
            Ok(nouveau) => {
                let ancien = self
                    .config
                    .jeu_mut(id)
                    .and_then(|j| j.icone_perso.replace(nouveau));
                if let Some(a) = ancien {
                    let _ = std::fs::remove_file(self.dossiers.icones.join(a));
                }
                self.icones.invalider(id);
                self.sauver();
            }
            Err(e) => self.notifier_erreur(e),
        }
    }

    fn retirer(&mut self, id: u64) {
        if let Some(pos) = self.config.jeux.iter().position(|j| j.id == id) {
            let jeu = self.config.jeux.remove(pos);
            if let Some(nom) = jeu.icone_perso {
                let _ = std::fs::remove_file(self.dossiers.icones.join(nom));
            }
            for nom in [jeu.jaquette, jeu.banniere].into_iter().flatten() {
                let _ = std::fs::remove_file(self.dossiers.jaquettes.join(nom));
            }
            self.icones.invalider(id);
            self.manquants.remove(&id);
            self.sauver();
        }
    }

    /// Garde-fous avant d'envoyer un dossier à la corbeille.
    fn verifier_suppression(&self, jeu: &Jeu) -> Result<PathBuf, String> {
        let cible = jeu
            .dossier_a_supprimer()
            .ok_or("Impossible de déterminer le dossier du jeu.")?;

        if !cible.exists() {
            return Err("Le dossier du jeu n'existe plus sur le disque.".into());
        }
        if !cible.is_absolute() {
            return Err("Chemin relatif refusé par sécurité.".into());
        }
        let profondeur = cible
            .components()
            .filter(|c| matches!(c, std::path::Component::Normal(_)))
            .count();
        if profondeur < 2 {
            return Err(format!(
                "« {} » est trop proche de la racine du disque.",
                cible.display()
            ));
        }
        if !jeu.exe.starts_with(&cible) && jeu.exe != cible {
            return Err("L'exécutable n'est pas dans le dossier ciblé.".into());
        }

        let mut proteges: Vec<PathBuf> = vec![self.dossiers.racine.clone()];
        if let Some(d) = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(Path::to_path_buf))
        {
            proteges.push(d);
        }
        for var in [
            "USERPROFILE",
            "HOME",
            "PROGRAMFILES",
            "PROGRAMFILES(X86)",
            "WINDIR",
            "PUBLIC",
        ] {
            if let Some(v) = std::env::var_os(var) {
                proteges.push(PathBuf::from(v));
            }
        }
        if let Some(d) = &self.config.dernier_dossier_scan {
            proteges.push(d.clone());
            proteges.push(scan::resoudre_racine(d));
        }
        let cle = cle_chemin(&cible);
        for p in &proteges {
            let cp = cle_chemin(p);
            // Refus si la cible EST un dossier protégé ou le CONTIENT.
            if cle == cp || cp.starts_with(&format!("{cle}\\")) {
                return Err(format!("« {} » est un dossier protégé.", cible.display()));
            }
        }

        // Un autre jeu de la bibliothèque partage-t-il ce dossier ?
        let partage = self
            .config
            .jeux
            .iter()
            .any(|j| j.id != jeu.id && j.exe.starts_with(&cible) && cible.is_dir());
        if partage {
            return Err("D'autres jeux de la bibliothèque se trouvent dans ce dossier.".into());
        }
        Ok(cible)
    }

    fn supprimer(&mut self, id: u64, cible: PathBuf, ctx: &egui::Context) {
        let nom = self.jeu(id).map(|j| j.nom.clone()).unwrap_or_default();
        let message = format!("Suppression de « {nom} » en cours…");
        self.retirer(id);
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let r = platform::mettre_a_la_corbeille(&cible)
                .map(|_| format!("« {nom} » a été mis à la corbeille."))
                .map_err(|e| format!("Échec de la suppression de « {nom} » : {e}"));
            let _ = tx.send(r);
            ctx.request_repaint();
        });
        self.suppressions.push(rx);
        self.notifier(message);
    }

    fn suivre_suppressions(&mut self) {
        let mut messages = Vec::new();
        self.suppressions.retain(|rx| match rx.try_recv() {
            Ok(m) => {
                messages.push(m);
                false
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => true,
            Err(_) => false,
        });
        for m in messages {
            match m {
                Ok(t) => self.notifier(t),
                Err(t) => self.notifier_erreur(t),
            }
        }
    }

    fn vider_cache(&mut self) {
        if let Ok(entrees) = std::fs::read_dir(&self.dossiers.cache) {
            for e in entrees.flatten() {
                let _ = std::fs::remove_file(e.path());
            }
        }
        self.icones.tout_invalider();
        self.notifier("Cache d'icônes vidé, les icônes vont être ré-extraites.");
    }

    // ------------------------------------------------------------------
    // Sauvegardes
    // ------------------------------------------------------------------

    fn rafraichir_sauvegardes(&mut self) {
        self.sauvegardes = sauvegarde::lister(&self.dossiers.sauvegardes);
    }

    /// Remplace toute la configuration (restauration ou import).
    fn remplacer_config(&mut self, mut nouvelle: Config, ctx: &egui::Context) {
        if let Err(e) = sauvegarde::sauvegarde_immediate(&self.dossiers, "avant-remplacement") {
            self.notifier_erreur(format!(
                "Copie de sécurité impossible, opération annulée : {e}"
            ));
            return;
        }
        // Les icônes personnalisées absentes de ce PC sont oubliées.
        for j in &mut nouvelle.jeux {
            if j
                .icone_perso
                .as_ref()
                .is_some_and(|n| !self.dossiers.icones.join(n).is_file())
            {
                j.icone_perso = None;
            }
            let dossier = &self.dossiers.jaquettes;
            for image in [&mut j.jaquette, &mut j.banniere] {
                if image.as_ref().is_some_and(|n| !dossier.join(n).is_file()) {
                    *image = None;
                }
            }
        }
        self.config = nouvelle;
        self.filtre_tag = None;
        self.filtre_statut = None;
        self.renommage = None;
        self.icones.tout_invalider();
        self.verifier_manquants();
        self.appliquer_theme(ctx);
        self.sauver();
        self.rafraichir_sauvegardes();
    }

    fn exporter(&mut self) {
        let nom = format!(
            "omni_launcher_{}.json",
            chrono::Local::now().format("%Y-%m-%d")
        );
        let Some(chemin) = rfd::FileDialog::new()
            .set_title("Exporter la bibliothèque")
            .set_file_name(nom)
            .add_filter("Configuration Omni Launcher", &["json"])
            .save_file()
        else {
            return;
        };
        match sauvegarde::exporter(&self.config, &chemin) {
            Ok(()) => self.notifier(format!(
                "Bibliothèque exportée ({} jeux).",
                self.config.jeux.len()
            )),
            Err(e) => self.notifier_erreur(format!("Export impossible : {e}")),
        }
    }

    fn choisir_import(&mut self) {
        let Some(chemin) = rfd::FileDialog::new()
            .set_title("Importer une bibliothèque")
            .add_filter("Configuration Omni Launcher", &["json"])
            .pick_file()
        else {
            return;
        };
        match sauvegarde::lire(&chemin) {
            Ok(config) => {
                self.dialogue = Some(Dialogue::Importer {
                    config: Box::new(config),
                    fichier: chemin
                        .file_name()
                        .map(|f| f.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                });
            }
            Err(e) => self.notifier_erreur(e),
        }
    }

    // ------------------------------------------------------------------
    // Manette et clavier
    // ------------------------------------------------------------------

    /// Relève la manette. Les commandes reçues pendant que le launcher n'a
    /// pas le focus (en pleine partie, par exemple) sont ignorées.
    fn suivre_manette(&mut self, ctx: &egui::Context) {
        let actif = ctx.input(|i| {
            let v = i.viewport();
            v.focused != Some(false) && v.minimized != Some(true)
        });
        while let Ok(m) = self.manette.try_recv() {
            match m {
                MsgManette::Connectee(nom) => self.manette_connectee = Some(nom),
                MsgManette::Deconnectee => self.manette_connectee = None,
                MsgManette::Commande(c) if actif => self.commandes.push(c),
                MsgManette::Commande(_) => {}
            }
        }
    }

    fn basculer_mode_tv(&mut self, ctx: &egui::Context) {
        self.mode_tv = !self.mode_tv;
        self.renommage = None;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.mode_tv));
        if self.mode_tv {
            self.tv.entrer();
        }
    }

    fn raccourcis_clavier(&mut self, ctx: &egui::Context) {
        let (ctrl_f, f11, deposes) = ctx.input(|i| {
            (
                i.modifiers.command && i.key_pressed(Key::F),
                i.key_pressed(Key::F11),
                i.raw
                    .dropped_files
                    .iter()
                    .map(|f| f.path().to_path_buf())
                    .filter(|p| !p.as_os_str().is_empty())
                    .collect::<Vec<_>>(),
            )
        });
        if f11 {
            self.commandes.push(Commande::BasculerPleinEcran);
        }
        if ctrl_f && !self.mode_tv {
            self.vue = Vue::Bibliotheque;
            self.focus_recherche = true;
        }
        if !deposes.is_empty() {
            let (dossiers, fichiers): (Vec<PathBuf>, Vec<PathBuf>) =
                deposes.into_iter().partition(|p| p.is_dir());
            let exes: Vec<PathBuf> = fichiers
                .into_iter()
                .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe")))
                .collect();
            if !exes.is_empty() {
                self.ajouter_exes(exes);
            }
            if let Some(d) = dossiers.into_iter().next() {
                self.lancer_scan(d, ctx);
            }
        }
    }

    fn traiter_commandes(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        for c in std::mem::take(&mut self.commandes) {
            if c == Commande::BasculerPleinEcran {
                self.basculer_mode_tv(ctx);
            } else if self.mode_tv {
                self.commande_tv(c, ctx, actions);
            }
        }
    }

    // ------------------------------------------------------------------
    // Interface
    // ------------------------------------------------------------------

    fn entete(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let ctx = ui.ctx().clone();
        let p = theme::p();
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.add_space(14.0);
            match self.vue {
                Vue::Bibliotheque => {
                    ui.heading("Bibliothèque");
                    ui.label(
                        RichText::new(format!("{} jeux", self.config.jeux.len()))
                            .color(p.fg_muted),
                    );
                }
                Vue::Parametres | Vue::Statistiques => {
                    if ui.button("⬅  Retour à la bibliothèque").clicked() {
                        self.vue = Vue::Bibliotheque;
                    }
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(14.0);
                if self.vue == Vue::Bibliotheque {
                    if ui.button("⚙  Paramètres").clicked() {
                        self.rafraichir_sauvegardes();
                        self.vue = Vue::Parametres;
                    }
                    self.bouton_maj(ui);
                    if ui
                        .button("📊")
                        .on_hover_text("Statistiques de jeu")
                        .clicked()
                    {
                        self.vue = Vue::Statistiques;
                    }
                    if ui
                        .button("🎮")
                        .on_hover_text("Mode plein écran à la manette (F11 ou bouton Start)")
                        .clicked()
                    {
                        self.commandes.push(Commande::BasculerPleinEcran);
                    }
                    let scan_btn = ui.add_enabled(
                        self.scan.is_none(),
                        egui::Button::new("🗁  Scanner"),
                    );
                    if scan_btn
                        .on_hover_text("Ajoute automatiquement tous les jeux d'un dossier")
                        .clicked()
                    {
                        self.choisir_dossier_scan(&ctx);
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("+  Ajouter un jeu").color(p.sur_accent()),
                            )
                            .fill(p.accent),
                        )
                        .clicked()
                    {
                        self.choisir_exe();
                    }

                    ui.add_space(8.0);
                    egui::ComboBox::from_id_salt("affichage")
                        .selected_text(match self.config.affichage {
                            Affichage::Grille => "Icônes",
                            Affichage::Jaquettes => "Jaquettes",
                            Affichage::Liste => "Liste",
                        })
                        .show_ui(ui, |ui| {
                            let avant = self.config.affichage;
                            ui.selectable_value(&mut self.config.affichage, Affichage::Grille, "Icônes");
                            ui.selectable_value(
                                &mut self.config.affichage,
                                Affichage::Jaquettes,
                                "Jaquettes",
                            );
                            ui.selectable_value(&mut self.config.affichage, Affichage::Liste, "Liste");
                            if avant != self.config.affichage {
                                self.sauver();
                            }
                        });
                    egui::ComboBox::from_id_salt("tri")
                        .selected_text(match self.config.tri {
                            Tri::Nom => "Tri : nom",
                            Tri::DernierLancement => "Tri : récents",
                            Tri::DateAjout => "Tri : ajout",
                            Tri::TempsDeJeu => "Tri : temps de jeu",
                        })
                        .show_ui(ui, |ui| {
                            let avant = self.config.tri;
                            ui.selectable_value(&mut self.config.tri, Tri::Nom, "Nom");
                            ui.selectable_value(
                                &mut self.config.tri,
                                Tri::DernierLancement,
                                "Joués récemment",
                            );
                            ui.selectable_value(
                                &mut self.config.tri,
                                Tri::DateAjout,
                                "Ajoutés récemment",
                            );
                            ui.selectable_value(
                                &mut self.config.tri,
                                Tri::TempsDeJeu,
                                "Les plus joués",
                            );
                            if avant != self.config.tri {
                                self.sauver();
                            }
                        });

                    let largeur = (ui.available_width() - 10.0).clamp(140.0, 320.0);
                    let rech = ui.add(
                        egui::TextEdit::singleline(&mut self.recherche)
                            .hint_text("🔍  Rechercher (Ctrl+F)")
                            .desired_width(largeur)
                            .margin(egui::vec2(10.0, 7.0)),
                    );
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
                            let (fav, autres) = self.listes_visibles();
                            if let Some(&id) = fav.first().or(autres.first()) {
                                actions.push(Action::Lancer(id));
                            }
                        }
                    }
                }
            });
        });
        ui.add_space(10.0);
    }

    /// Rangée de filtres par statut et par tag, sous l'en-tête.
    fn filtres(&mut self, ui: &mut egui::Ui) {
        let tags = self.config.tags();
        let statuts = self.config.statuts();
        if self
            .filtre_statut
            .is_some_and(|f| !statuts.iter().any(|(s, _)| *s == f))
        {
            self.filtre_statut = None;
        }
        if tags.is_empty() && statuts.is_empty() {
            self.filtre_tag = None;
            return;
        }
        if self
            .filtre_tag
            .as_ref()
            .is_some_and(|f| !tags.iter().any(|(t, _)| t.eq_ignore_ascii_case(f)))
        {
            self.filtre_tag = None;
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            let aucun = self.filtre_tag.is_none() && self.filtre_statut.is_none();
            if puce(ui, "Tous", aucun).clicked() {
                self.filtre_tag = None;
                self.filtre_statut = None;
            }
            for (statut, n) in statuts {
                let actif = self.filtre_statut == Some(statut);
                if puce(ui, &format!("{}  {n}", statut.libelle()), actif).clicked() {
                    self.filtre_statut = if actif { None } else { Some(statut) };
                }
            }
            for (tag, n) in tags {
                let actif = self
                    .filtre_tag
                    .as_ref()
                    .is_some_and(|f| f.eq_ignore_ascii_case(&tag));
                if puce(ui, &format!("{tag}  {n}"), actif).clicked() {
                    self.filtre_tag = if actif { None } else { Some(tag) };
                }
            }
        });
        ui.add_space(12.0);
    }

    fn bibliotheque(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let ctx = ui.ctx().clone();
        let p = theme::p();
        if self.config.jeux.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.25);
                ui.label(RichText::new("Aucun jeu dans votre bibliothèque").size(22.0).color(p.fg));
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Ajoutez un exécutable, scannez votre dossier de jeux, ou glissez-déposez-les ici.")
                        .color(p.fg_muted),
                );
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    let largeur = 420.0;
                    ui.add_space(((ui.available_width() - largeur) / 2.0).max(0.0));
                    if ui
                        .add(egui::Button::new(RichText::new("+  Ajouter un jeu").color(p.sur_accent())).fill(p.accent))
                        .clicked()
                    {
                        self.choisir_exe();
                    }
                    if ui.add_enabled(self.scan.is_none(), egui::Button::new("🗁  Scanner un dossier")).clicked() {
                        self.choisir_dossier_scan(&ctx);
                    }
                });
            });
            return;
        }

        let (favoris, autres) = self.listes_visibles();
        let filtre_actif = !self.recherche.trim().is_empty()
            || self.filtre_tag.is_some()
            || self.filtre_statut.is_some();

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(4.0);
            egui::Frame::new().inner_margin(egui::Margin::symmetric(24, 0)).show(ui, |ui| {
                self.filtres(ui);

                if !filtre_actif && self.config.afficher_recents {
                    self.recents(ui, actions);
                }

                if filtre_actif && favoris.is_empty() && autres.is_empty() {
                    ui.add_space(40.0);
                    ui.vertical_centered(|ui| {
                        let texte = if self.recherche.trim().is_empty() {
                            "Aucun jeu avec ce filtre.".to_string()
                        } else {
                            format!("Aucun jeu ne correspond à « {} ».", self.recherche.trim())
                        };
                        ui.label(RichText::new(texte).color(p.fg_muted));
                    });
                    return;
                }

                if !filtre_actif || !favoris.is_empty() {
                    titre_section(ui, "Vos favoris ⭐");
                    if favoris.is_empty() {
                        ui.label(
                            RichText::new("Aucun favori pour le moment. Clic droit sur un jeu ➔ Ajouter aux favoris.")
                                .italics()
                                .color(p.fg_muted),
                        );
                        ui.add_space(16.0);
                    } else {
                        self.afficher_jeux(ui, &favoris, actions);
                    }
                }

                if !autres.is_empty() {
                    ui.add_space(12.0);
                    let recherche = !self.recherche.trim().is_empty();
                    let titre = match (&self.filtre_tag, self.filtre_statut) {
                        _ if recherche => "Résultats".to_string(),
                        (Some(t), _) => t.clone(),
                        (None, Some(s)) => s.libelle().to_string(),
                        (None, None) => "Autres jeux".to_string(),
                    };
                    titre_section(ui, &titre);
                    self.afficher_jeux(ui, &autres, actions);
                }
                ui.add_space(24.0);
            });
        });
    }

    fn afficher_jeux(&mut self, ui: &mut egui::Ui, ids: &[u64], actions: &mut Vec<Action>) {
        match self.config.affichage {
            Affichage::Grille | Affichage::Jaquettes => self.grille(ui, ids, actions),
            Affichage::Liste => self.liste(ui, ids, actions),
        }
    }

    fn grille(&mut self, ui: &mut egui::Ui, ids: &[u64], actions: &mut Vec<Action>) {
        let cote = self.config.taille_tuile;
        let hauteur = match self.config.affichage {
            Affichage::Jaquettes => cote * RATIO_JAQUETTE,
            Affichage::Grille | Affichage::Liste => cote,
        };
        let taille = Vec2::new(cote + 24.0, hauteur + 56.0);
        let ecart = 14.0;
        let colonnes =
            (((ui.available_width() + ecart) / (taille.x + ecart)).floor() as usize).max(1);

        for ligne in ids.chunks(colonnes) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = ecart;
                for &id in ligne {
                    self.tuile(ui, id, taille, actions);
                }
            });
            ui.add_space(ecart);
        }
    }

    /// Dessine l'icône d'un jeu (ou son initiale tant qu'elle n'est pas prête).
    fn dessiner_icone(&mut self, ui: &egui::Ui, jeu: &Jeu, zone: egui::Rect, manquant: bool) {
        let p = theme::p();
        let cote = zone.width();
        let peintre = ui.painter_at(zone.expand(2.0));
        match self.icones.texture(jeu.id, &jeu.exe, jeu.icone_perso.as_deref()) {
            Some(t) => {
                let ts = t.size_vec2();
                let echelle = (cote / ts.x).min(cote / ts.y);
                let r = egui::Rect::from_center_size(zone.center(), ts * echelle);
                let mut img = egui::Image::new(&t).corner_radius(CornerRadius::same(8));
                if manquant {
                    img = img.tint(Color32::from_gray(90));
                }
                img.paint_at(ui, r);
            }
            None => {
                peintre.rect_filled(zone.shrink(6.0), CornerRadius::same(14), p.bg_btn);
                peintre.rect_stroke(
                    zone.shrink(6.0),
                    CornerRadius::same(14),
                    Stroke::new(1.5, p.bordure),
                    egui::StrokeKind::Inside,
                );
                let initiale = jeu
                    .nom
                    .chars()
                    .next()
                    .unwrap_or('?')
                    .to_uppercase()
                    .to_string();
                peintre.text(
                    zone.center(),
                    Align2::CENTER_CENTER,
                    initiale,
                    FontId::new(cote * 0.42, theme::gras()),
                    p.fg_muted,
                );
            }
        }
        if manquant && cote >= TAILLE_TUILE_MIN {
            peintre.text(
                zone.center_bottom() - Vec2::new(0.0, 8.0),
                Align2::CENTER_BOTTOM,
                "Introuvable",
                FontId::new(13.0, theme::gras()),
                p.danger,
            );
        }
    }

    /// Dessine la jaquette d'un jeu, ou à défaut son icône sur une carte.
    fn dessiner_jaquette(&mut self, ui: &egui::Ui, jeu: &Jeu, zone: egui::Rect, manquant: bool) {
        let p = theme::p();
        let arrondi = CornerRadius::same(8);
        let texture = jeu
            .jaquette
            .as_deref()
            .and_then(|f| self.icones.visuel(jeu.id, Genre::Jaquette, f));
        match texture {
            Some(t) => {
                let uv = icons::uv_couvrir(t.size_vec2(), zone.size());
                let mut img = egui::Image::new(&t).uv(uv).corner_radius(arrondi);
                if manquant {
                    img = img.tint(Color32::from_gray(90));
                }
                img.paint_at(ui, zone);
                if manquant {
                    ui.painter().text(
                        zone.center_bottom() - Vec2::new(0.0, 8.0),
                        Align2::CENTER_BOTTOM,
                        "Introuvable",
                        FontId::new(13.0, theme::gras()),
                        p.danger,
                    );
                }
            }
            None => {
                ui.painter().rect_filled(zone, arrondi, p.bg_btn);
                let icone = egui::Rect::from_center_size(zone.center(), Vec2::splat(zone.width() * 0.62));
                self.dessiner_icone(ui, jeu, icone, manquant);
            }
        }
    }

    /// Champ de saisie du renommage en cours.
    fn champ_renommage(
        &mut self,
        ui: &mut egui::Ui,
        zone: egui::Rect,
        alignement: Align2,
        actions: &mut Vec<Action>,
    ) {
        let Some(r) = self.renommage.as_mut() else {
            return;
        };
        let id = r.id;
        let edit = ui.put(
            zone,
            egui::TextEdit::singleline(&mut r.texte)
                .align(alignement)
                .font(FontId::new(14.0, theme::gras())),
        );
        if r.premiere_frame {
            edit.request_focus();
            if let Some(mut etat) = egui::TextEdit::load_state(ui.ctx(), edit.id) {
                etat.cursor
                    .set_char_range(Some(egui::text::CCursorRange::select_all_str(&r.texte)));
                etat.store(ui.ctx(), edit.id);
            }
            r.premiere_frame = false;
        } else if edit.lost_focus() {
            if ui.input(|i| i.key_pressed(Key::Enter)) {
                actions.push(Action::Renommer(id, r.texte.clone()));
            }
            self.renommage = None;
        }
    }

    fn tuile(&mut self, ui: &mut egui::Ui, id: u64, taille: Vec2, actions: &mut Vec<Action>) {
        let Some(jeu) = self.jeu(id).cloned() else {
            return;
        };
        let p = theme::p();
        let manquant = self.manquants.contains(&id);
        let cote = self.config.taille_tuile;

        let (rect, reponse) = ui.allocate_exact_size(taille, Sense::click());
        if !ui.is_rect_visible(rect) {
            return; // Tuile hors écran : rien à dessiner ni à charger.
        }
        let survol = reponse.hovered() || reponse.context_menu_opened();
        let peintre = ui.painter_at(rect.expand(2.0));

        if survol {
            peintre.rect_filled(rect, CornerRadius::same(10), p.bg_tuile_hover);
        }

        let zone_icone = egui::Rect::from_min_size(
            rect.min + Vec2::new(12.0, 10.0),
            Vec2::new(cote, taille.y - 56.0),
        );
        if self.config.affichage == Affichage::Jaquettes {
            self.dessiner_jaquette(ui, &jeu, zone_icone, manquant);
        } else {
            self.dessiner_icone(ui, &jeu, zone_icone, manquant);
        }
        marquer_statut(&peintre, zone_icone.left_top() + Vec2::splat(10.0), jeu.statut);

        // Nom (ou champ de renommage)
        let zone_nom = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 4.0, zone_icone.bottom() + 6.0),
            egui::pos2(rect.right() - 4.0, rect.bottom()),
        );

        let en_renommage = self.renommage.as_ref().is_some_and(|r| r.id == id);
        if en_renommage {
            let zone = egui::Rect::from_min_size(zone_nom.min, Vec2::new(zone_nom.width(), 28.0));
            self.champ_renommage(ui, zone, Align2::CENTER_CENTER, actions);
        } else {
            let mut job = egui::text::LayoutJob::simple(
                jeu.nom.clone(),
                FontId::new(14.0, theme::gras()),
                if manquant { p.fg_muted } else { p.fg },
                zone_nom.width(),
            );
            job.halign = egui::Align::Center;
            job.wrap.max_rows = 2;
            job.wrap.break_anywhere = false;
            job.wrap.overflow_character = Some('…');
            let galley = ui.painter().layout_job(job);
            peintre.galley(egui::pos2(zone_nom.center().x, zone_nom.top()), galley, p.fg);
        }

        interactions_jeu(reponse, &jeu, actions);
    }

    fn fenetres(&mut self, ctx: &egui::Context) {
        self.fenetre_scan(ctx);
        self.fenetre_revue(ctx);
        self.fenetre_dialogue(ctx);
        self.fenetre_visuels(ctx);
        self.fenetre_lot(ctx);
    }

    fn fenetre_scan(&mut self, ctx: &egui::Context) {
        let Some(scan) = &self.scan else { return };
        let p = theme::p();
        let annuler = egui::Modal::new(egui::Id::unique("scan")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.heading("Scan en cours…");
            ui.add_space(8.0);
            let progression = if scan.total > 0 {
                scan.fait as f32 / scan.total as f32
            } else {
                0.0
            };
            ui.add(
                egui::ProgressBar::new(progression)
                    .show_percentage()
                    .fill(p.accent.linear_multiply(0.6)),
            );
            ui.label(RichText::new(&scan.nom).color(p.fg_muted));
            ui.add_space(8.0);
            ui.button("Annuler").clicked()
        });
        if annuler.inner {
            scan.annule.store(true, Ordering::Relaxed);
        }
    }

    fn fenetre_revue(&mut self, ctx: &egui::Context) {
        let Some(revue) = &mut self.revue else { return };
        let p = theme::p();
        let mut fermer = false;
        let mut importer = false;

        egui::Modal::new(egui::Id::unique("revue")).show(ctx, |ui| {
            ui.set_width(720.0f32.min(ctx.content_rect().width() - 60.0));
            ui.heading("Jeux trouvés");
            ui.label(
                RichText::new(
                    "L'exécutable le plus probable de chaque jeu est pré-coché. \
                     Vous pouvez corriger les noms avant d'importer.",
                )
                .color(p.fg_muted),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Tout cocher").clicked() {
                    revue.candidats.iter_mut().for_each(|c| c.coche = true);
                }
                if ui.button("Tout décocher").clicked() {
                    revue.candidats.iter_mut().for_each(|c| c.coche = false);
                }
                if ui.button("Uniquement les principaux").clicked() {
                    let mut vus = HashSet::new();
                    for c in revue.candidats.iter_mut() {
                        c.coche = vus.insert(c.groupe.clone()) || c.groupe == "(fichiers isolés)";
                    }
                }
            });
            ui.separator();

            let hauteur = (ctx.content_rect().height() - 260.0).max(160.0);
            egui::ScrollArea::vertical()
                .max_height(hauteur)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    let mut groupe_courant = String::new();
                    for (i, c) in revue.candidats.iter_mut().enumerate() {
                        if c.groupe != groupe_courant {
                            groupe_courant = c.groupe.clone();
                            ui.add_space(6.0);
                            ui.label(RichText::new(&c.groupe).strong().color(p.accent));
                        }
                        ui.push_id(i, |ui| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut c.coche, "");
                                ui.add_enabled(
                                    c.coche,
                                    egui::TextEdit::singleline(&mut c.nom).desired_width(220.0),
                                );
                                let rel = c
                                    .exe
                                    .file_name()
                                    .map(|f| f.to_string_lossy().into_owned())
                                    .unwrap_or_default();
                                ui.label(RichText::new(rel).small().color(p.fg_muted))
                                    .on_hover_text(c.exe.display().to_string());
                            });
                        });
                    }
                });

            ui.separator();
            let n = revue.candidats.iter().filter(|c| c.coche).count();
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        n > 0,
                        egui::Button::new(
                            RichText::new(format!("Importer {n} jeu(x)")).color(p.sur_accent()),
                        )
                        .fill(p.accent),
                    )
                    .clicked()
                {
                    importer = true;
                }
                if ui.button("Annuler").clicked() {
                    fermer = true;
                }
            });
        });

        if importer {
            let candidats = self.revue.take().map(|r| r.candidats).unwrap_or_default();
            self.importer_revue(candidats);
        } else if fermer {
            self.revue = None;
        }
    }

    fn fenetre_dialogue(&mut self, ctx: &egui::Context) {
        let Some(dialogue) = &mut self.dialogue else {
            return;
        };
        let p = theme::p();
        let mut fermer = false;
        let mut action: Option<ActionDialogue> = None;
        // Action appliquée sans fermer la boîte de dialogue (édition des tags).
        let mut modification: Option<ActionDialogue> = None;
        // Une commande « Retour » de la manette ferme la boîte de dialogue.
        if let Some(i) = self.commandes.iter().position(|c| *c == Commande::Retour) {
            self.commandes.remove(i);
            fermer = true;
        }

        let nom = |id: &u64, jeux: &[Jeu]| {
            jeux.iter()
                .find(|j| j.id == *id)
                .map(|j| j.nom.clone())
                .unwrap_or_default()
        };
        let tous_tags = self.config.tags();
        let jeux = &self.config.jeux;

        let reponse = egui::Modal::new(egui::Id::unique("dialogue")).show(ctx, |ui| {
            ui.set_max_width(480.0);
            match dialogue {
                Dialogue::Retirer(id) => {
                    let id = *id;
                    ui.heading("Retirer du launcher");
                    ui.label(format!(
                        "Retirer « {} » de la bibliothèque ?",
                        nom(&id, jeux)
                    ));
                    ui.label(
                        RichText::new("Le jeu reste installé sur votre ordinateur.")
                            .color(p.fg_muted),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Retirer").clicked() {
                            action = Some(Box::new(move |app, _| app.retirer(id)));
                        }
                        if ui.button("Annuler").clicked() {
                            fermer = true;
                        }
                    });
                }
                Dialogue::Supprimer { id, cible } => {
                    let (id, cible) = (*id, cible.clone());
                    ui.heading(RichText::new("Supprimer le jeu du disque").color(p.danger));
                    ui.label(format!(
                        "Le dossier suivant de « {} » va être envoyé à la corbeille :",
                        nom(&id, jeux)
                    ));
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(cible.display().to_string())
                            .color(p.accent)
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Vous pourrez le restaurer depuis la corbeille de Windows.")
                            .color(p.fg_muted),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Mettre à la corbeille")
                                        .color(theme::texte_sur(p.danger)),
                                )
                                .fill(p.danger),
                            )
                            .clicked()
                        {
                            action = Some(Box::new(move |app, ctx| app.supprimer(id, cible, ctx)));
                        }
                        if ui.button("Annuler").clicked() {
                            fermer = true;
                        }
                    });
                }
                Dialogue::SuppressionRefusee { id, raison } => {
                    let id = *id;
                    ui.heading("Suppression impossible");
                    ui.label(format!(
                        "Par sécurité, le dossier de « {} » ne sera pas supprimé :",
                        nom(&id, jeux)
                    ));
                    ui.label(RichText::new(raison.as_str()).color(p.danger));
                    ui.label(
                        RichText::new("Vous pouvez le supprimer vous-même depuis l'Explorateur.")
                            .color(p.fg_muted),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Ouvrir l'emplacement").clicked() {
                            action = Some(Box::new(move |app, ctx| {
                                app.executer(Action::OuvrirEmplacement(id), ctx)
                            }));
                        }
                        if ui.button("Retirer seulement du launcher").clicked() {
                            action = Some(Box::new(move |app, _| app.retirer(id)));
                        }
                        if ui.button("Fermer").clicked() {
                            fermer = true;
                        }
                    });
                }
                Dialogue::Introuvable(id) => {
                    let id = *id;
                    ui.heading("Jeu introuvable");
                    ui.label(format!(
                        "L'exécutable de « {} » n'existe plus :",
                        nom(&id, jeux)
                    ));
                    if let Some(j) = jeux.iter().find(|j| j.id == id) {
                        ui.label(RichText::new(j.exe.display().to_string()).color(p.fg_muted));
                    }
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Retirer du launcher").clicked() {
                            action = Some(Box::new(move |app, _| app.retirer(id)));
                        }
                        if ui.button("Localiser l'exécutable…").clicked() {
                            action = Some(Box::new(move |app, _| app.relocaliser(id)));
                        }
                        if ui.button("Fermer").clicked() {
                            fermer = true;
                        }
                    });
                }
                Dialogue::RetirerIntrouvables => {
                    ui.heading("Retirer les jeux introuvables");
                    let noms: Vec<String> = jeux
                        .iter()
                        .filter(|j| !j.exe.is_file())
                        .map(|j| j.nom.clone())
                        .collect();
                    ui.label(format!(
                        "{} jeu(x) seront retirés de la bibliothèque :",
                        noms.len()
                    ));
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            for n in &noms {
                                ui.label(RichText::new(format!("•  {n}")).color(p.fg_muted));
                            }
                        });
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Retirer").clicked() {
                            action = Some(Box::new(|app, _| {
                                let ids: Vec<u64> = app
                                    .config
                                    .jeux
                                    .iter()
                                    .filter(|j| !j.exe.is_file())
                                    .map(|j| j.id)
                                    .collect();
                                for id in ids {
                                    app.retirer(id);
                                }
                            }));
                        }
                        if ui.button("Annuler").clicked() {
                            fermer = true;
                        }
                    });
                }
                Dialogue::Arguments { id, texte } => {
                    let id = *id;
                    ui.heading("Arguments de lancement");
                    ui.label(
                        RichText::new(format!(
                            "Pour « {} » (ex. -windowed -nointro)",
                            nom(&id, jeux)
                        ))
                        .color(p.fg_muted),
                    );
                    let r = ui.add(egui::TextEdit::singleline(texte).desired_width(440.0));
                    let entree = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Enregistrer").clicked() || entree {
                            let t = texte.trim().to_string();
                            action = Some(Box::new(move |app, _| {
                                if let Some(j) = app.config.jeu_mut(id) {
                                    j.arguments = t;
                                }
                                app.sauver();
                            }));
                        }
                        if ui.button("Annuler").clicked() {
                            fermer = true;
                        }
                    });
                }
                Dialogue::Tags { id, saisie } => {
                    let id = *id;
                    let Some(jeu) = jeux.iter().find(|j| j.id == id) else {
                        fermer = true;
                        return;
                    };
                    ui.heading("Tags");
                    ui.label(RichText::new(format!("Pour « {} »", jeu.nom)).color(p.fg_muted));
                    ui.add_space(6.0);

                    ui.label(RichText::new("Tags du jeu").strong());
                    if jeu.tags.is_empty() {
                        ui.label(RichText::new("Aucun tag pour l'instant.").italics().color(p.fg_muted));
                    }
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        for t in &jeu.tags {
                            if puce(ui, &format!("{t}  ✕"), true)
                                .on_hover_text("Retirer ce tag")
                                .clicked()
                            {
                                let t = t.clone();
                                modification = Some(Box::new(move |app, _| {
                                    if let Some(j) = app.config.jeu_mut(id) {
                                        j.retirer_tag(&t);
                                    }
                                    app.sauver();
                                }));
                            }
                        }
                    });

                    ui.add_space(8.0);
                    ui.label(RichText::new("Nouveau tag").strong());
                    let mut ajouter = false;
                    ui.horizontal(|ui| {
                        let r = ui.add(
                            egui::TextEdit::singleline(saisie)
                                .hint_text("Coop, Rétro, À finir…")
                                .desired_width(280.0),
                        );
                        if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                            ajouter = true;
                            r.request_focus();
                        }
                        if ui.add_enabled(!saisie.trim().is_empty(), egui::Button::new("Ajouter")).clicked() {
                            ajouter = true;
                        }
                    });
                    if ajouter && !saisie.trim().is_empty() {
                        let t = std::mem::take(saisie);
                        modification = Some(Box::new(move |app, _| {
                            if let Some(j) = app.config.jeu_mut(id) {
                                j.ajouter_tag(&t);
                            }
                            app.sauver();
                        }));
                    }

                    let autres: Vec<&String> = tous_tags
                        .iter()
                        .map(|(t, _)| t)
                        .filter(|t| !jeu.a_tag(t))
                        .collect();
                    if !autres.is_empty() {
                        ui.add_space(8.0);
                        ui.label(RichText::new("Tags existants").strong());
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                            for t in autres {
                                if puce(ui, &format!("+ {t}"), false).clicked() {
                                    let t = t.clone();
                                    modification = Some(Box::new(move |app, _| {
                                        if let Some(j) = app.config.jeu_mut(id) {
                                            j.ajouter_tag(&t);
                                        }
                                        app.sauver();
                                    }));
                                }
                            }
                        });
                    }
                    ui.add_space(12.0);
                    if ui.button("Terminé").clicked() {
                        fermer = true;
                    }
                }
                Dialogue::Notes { id, statut, texte } => {
                    let id = *id;
                    ui.heading("Statut et notes");
                    ui.label(RichText::new(format!("Pour « {} »", nom(&id, jeux))).color(p.fg_muted));
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        for s in Statut::TOUS {
                            if puce(ui, s.libelle(), *statut == s).clicked() {
                                *statut = s;
                            }
                        }
                    });
                    ui.add_space(8.0);
                    ui.add(
                        egui::TextEdit::multiline(texte)
                            .hint_text("Vos notes : codes, mods, où vous en êtes…")
                            .desired_width(440.0)
                            .desired_rows(6),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Enregistrer").clicked() {
                            let (s, t) = (*statut, texte.trim().to_string());
                            action = Some(Box::new(move |app, _| {
                                if let Some(j) = app.config.jeu_mut(id) {
                                    j.statut = s;
                                    j.notes = t;
                                }
                                app.sauver();
                            }));
                        }
                        if ui.button("Annuler").clicked() {
                            fermer = true;
                        }
                    });
                }
                Dialogue::RenommerTag { ancien, texte } => {
                    ui.heading("Renommer le tag");
                    ui.label(
                        RichText::new(format!("« {ancien} » sera renommé sur tous les jeux."))
                            .color(p.fg_muted),
                    );
                    let r = ui.add(egui::TextEdit::singleline(texte).desired_width(300.0));
                    let entree = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if (ui.button("Renommer").clicked() || entree) && !texte.trim().is_empty() {
                            let (a, n) = (ancien.clone(), texte.trim().to_string());
                            action = Some(Box::new(move |app, _| {
                                if app.filtre_tag.as_ref().is_some_and(|f| f.eq_ignore_ascii_case(&a)) {
                                    app.filtre_tag = Some(n.clone());
                                }
                                app.config.renommer_tag(&a, &n);
                                app.sauver();
                            }));
                        }
                        if ui.button("Annuler").clicked() {
                            fermer = true;
                        }
                    });
                }
                Dialogue::SupprimerTag(tag) => {
                    let n = jeux.iter().filter(|j| j.a_tag(tag)).count();
                    ui.heading("Supprimer le tag");
                    ui.label(format!("Retirer le tag « {tag} » de {n} jeu(x) ?"));
                    ui.label(RichText::new("Les jeux eux-mêmes ne sont pas touchés.").color(p.fg_muted));
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Supprimer le tag").clicked() {
                            let t = tag.clone();
                            action = Some(Box::new(move |app, _| {
                                app.config.supprimer_tag(&t);
                                app.sauver();
                            }));
                        }
                        if ui.button("Annuler").clicked() {
                            fermer = true;
                        }
                    });
                }
                Dialogue::Restaurer(info) => {
                    ui.heading("Restaurer une sauvegarde");
                    ui.label(format!(
                        "Remplacer la bibliothèque et les réglages actuels par la sauvegarde du {} ?",
                        sauvegarde::libelle_date(info.date)
                    ));
                    if let Some(n) = info.nb_jeux {
                        ui.label(RichText::new(format!("Elle contient {n} jeu(x).")).color(p.fg_muted));
                    }
                    ui.label(
                        RichText::new("Une copie de l'état actuel est faite avant, vous pourrez revenir en arrière.")
                            .color(p.fg_muted),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Restaurer").clicked() {
                            let chemin = info.chemin.clone();
                            action = Some(Box::new(move |app, ctx| match sauvegarde::lire(&chemin) {
                                Ok(c) => {
                                    app.remplacer_config(c, ctx);
                                    app.notifier("Sauvegarde restaurée.");
                                }
                                Err(e) => app.notifier_erreur(e),
                            }));
                        }
                        if ui.button("Annuler").clicked() {
                            fermer = true;
                        }
                    });
                }
                Dialogue::Importer { config, fichier } => {
                    let n = config.jeux.len();
                    let nouveaux = config
                        .jeux
                        .iter()
                        .filter(|j| !jeux.iter().any(|k| cle_chemin(&k.exe) == cle_chemin(&j.exe)))
                        .count();
                    ui.heading("Importer une bibliothèque");
                    ui.label(format!("« {fichier} » contient {n} jeu(x), dont {nouveaux} absent(s) de votre bibliothèque."));
                    ui.label(
                        RichText::new("Les icônes personnalisées ne font pas partie de l'export.")
                            .color(p.fg_muted),
                    );
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add_enabled(
                                nouveaux > 0,
                                egui::Button::new(
                                    RichText::new(format!("Ajouter les {nouveaux} jeu(x) manquant(s)"))
                                        .color(p.sur_accent()),
                                )
                                .fill(p.accent),
                            )
                            .clicked()
                        {
                            let c = (**config).clone();
                            action = Some(Box::new(move |app, _| {
                                let n = app.config.fusionner(c);
                                app.sauver();
                                app.verifier_manquants();
                                app.notifier(format!("{n} jeu(x) importé(s)."));
                            }));
                        }
                        if ui.button("Tout remplacer (bibliothèque et réglages)").clicked() {
                            let c = (**config).clone();
                            action = Some(Box::new(move |app, ctx| {
                                app.remplacer_config(c, ctx);
                                app.notifier("Bibliothèque importée.");
                            }));
                        }
                        if ui.button("Annuler").clicked() {
                            fermer = true;
                        }
                    });
                }
            }
        });

        if reponse.should_close() {
            fermer = true;
        }
        if let Some(m) = modification {
            m(self, ctx);
        }
        if let Some(a) = action {
            self.dialogue = None;
            a(self, ctx);
        } else if fermer {
            self.dialogue = None;
        }
    }

    /// Permet de pointer vers le nouvel emplacement d'un jeu déplacé.
    fn relocaliser(&mut self, id: u64) {
        let Some(fichier) = rfd::FileDialog::new()
            .set_title("Nouvel emplacement de l'exécutable")
            .add_filter("Exécutables", &["exe"])
            .pick_file()
        else {
            return;
        };
        if let Some(j) = self.config.jeu_mut(id) {
            j.exe = fichier;
            j.cible_suppression = None;
        }
        self.icones.invalider(id);
        self.sauver();
        self.verifier_manquants();
    }

    fn afficher_notification(&mut self, ctx: &egui::Context) {
        let Some(n) = &self.notification else { return };
        let p = theme::p();
        let ecoule = n.depuis.elapsed();
        if ecoule > DUREE_NOTIF {
            self.notification = None;
            return;
        }
        ctx.request_repaint_after(DUREE_NOTIF - ecoule);

        egui::Area::new(egui::Id::unique("notification"))
            .anchor(Align2::CENTER_BOTTOM, [0.0, -20.0])
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(p.bg_menu)
                    .stroke(Stroke::new(
                        1.0,
                        if n.erreur {
                            p.danger
                        } else {
                            p.accent.linear_multiply(0.5)
                        },
                    ))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(16, 10))
                    .show(ui, |ui| {
                        ui.label(RichText::new(&n.texte).color(if n.erreur { p.danger } else { p.fg }));
                    });
            });
    }
}

/// « 12 h 30 de jeu · 23 lancements · joué hier »
fn resume_jeu(jeu: &Jeu) -> String {
    let mut parties = vec![if jeu.temps_jeu > 0 {
        format!("{} de jeu", stats::duree(jeu.temps_jeu))
    } else {
        "Pas encore de temps de jeu".to_string()
    }];
    match jeu.nb_lancements {
        0 => {}
        1 => parties.push("1 lancement".into()),
        n => parties.push(format!("{n} lancements")),
    }
    if let Some(t) = jeu.dernier_lancement {
        parties.push(format!("joué {}", stats::il_y_a(t, maintenant())));
    }
    parties.join(" · ")
}

fn couleur_statut(statut: Statut) -> Color32 {
    let p = theme::p();
    match statut {
        Statut::Aucun | Statut::AJouer => p.fg_muted,
        Statut::EnCours => p.accent,
        Statut::Termine => Color32::from_rgb(80, 190, 120),
        Statut::Abandonne => p.danger,
    }
}

/// Pastille de couleur sur une tuile, si le jeu a un statut.
fn marquer_statut(peintre: &egui::Painter, centre: egui::Pos2, statut: Statut) {
    if statut != Statut::Aucun {
        let contour = Stroke::new(2.0, theme::p().bg_main);
        peintre.circle(centre, 6.0, couleur_statut(statut), contour);
    }
}

/// Couleur rendue partiellement transparente.
fn voile(couleur: Color32, opacite: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(couleur.r(), couleur.g(), couleur.b(), opacite)
}

/// Les notes sur une seule ligne, coupées si elles sont longues.
fn apercu_notes(notes: &str) -> String {
    let ligne = notes.split_whitespace().collect::<Vec<_>>().join(" ");
    if ligne.chars().count() <= APERCU_NOTES_MAX {
        return ligne;
    }
    let debut: String = ligne.chars().take(APERCU_NOTES_MAX).collect();
    format!("{}…", debut.trim_end())
}

fn infobulle_jeu(ui: &mut egui::Ui, jeu: &Jeu) {
    let p = theme::p();
    ui.label(RichText::new(resume_jeu(jeu)).small());
    if jeu.statut != Statut::Aucun {
        ui.label(
            RichText::new(jeu.statut.libelle())
                .small()
                .color(couleur_statut(jeu.statut)),
        );
    }
    if !jeu.tags.is_empty() {
        ui.label(RichText::new(format!("🏷 {}", jeu.tags.join(", "))).small());
    }
    if !jeu.notes.is_empty() {
        ui.label(RichText::new(format!("📝 {}", apercu_notes(&jeu.notes))).small());
    }
    ui.label(
        RichText::new(jeu.exe.display().to_string())
            .small()
            .color(p.fg_muted),
    );
}

fn menu_jeu(ui: &mut egui::Ui, jeu: &Jeu, actions: &mut Vec<Action>) {
    let p = theme::p();
    let id = jeu.id;
    ui.set_min_width(230.0);
    ui.label(RichText::new(&jeu.nom).strong().color(p.accent));
    ui.label(RichText::new(resume_jeu(jeu)).small().color(p.fg_muted));
    ui.separator();
    if ui.button("▶  Lancer").clicked() {
        actions.push(Action::Lancer(id));
        ui.close();
    }
    if ui.button("🗁  Ouvrir l'emplacement").clicked() {
        actions.push(Action::OuvrirEmplacement(id));
        ui.close();
    }
    ui.separator();
    let fav = if jeu.favori {
        "☆  Retirer des favoris"
    } else {
        "⭐  Ajouter aux favoris"
    };
    if ui.button(fav).clicked() {
        actions.push(Action::BasculerFavori(id));
        ui.close();
    }
    if ui.button("🏷  Tags…").clicked() {
        actions.push(Action::EditerTags(id));
        ui.close();
    }
    if ui.button("📝  Statut et notes…").clicked() {
        actions.push(Action::EditerNotes(id));
        ui.close();
    }
    ui.separator();
    if ui.button("✏  Renommer").clicked() {
        actions.push(Action::DebuterRenommage(id));
        ui.close();
    }
    if ui.button("🖼  Modifier l'icône…").clicked() {
        actions.push(Action::ChangerIcone(id));
        ui.close();
    }
    if jeu.icone_perso.is_some() && ui.button("↺  Rétablir l'icône d'origine").clicked() {
        actions.push(Action::RetablirIcone(id));
        ui.close();
    }
    if ui.button("🖼  Jaquette et bannière…").clicked() {
        actions.push(Action::EditerVisuels(id));
        ui.close();
    }
    if ui.button("⚙  Arguments de lancement…").clicked() {
        actions.push(Action::EditerArguments(id));
        ui.close();
    }
    ui.separator();
    if ui.button("➖  Retirer du launcher").clicked() {
        actions.push(Action::DemanderRetrait(id));
        ui.close();
    }
    if ui
        .button(RichText::new("🗑  Supprimer le jeu du disque…").color(p.danger))
        .clicked()
    {
        actions.push(Action::DemanderSuppression(id));
        ui.close();
    }
}

/// Double-clic, infobulle et menu contextuel, communs à toutes les vues.
fn interactions_jeu(reponse: egui::Response, jeu: &Jeu, actions: &mut Vec<Action>) {
    if reponse.double_clicked() {
        actions.push(Action::Lancer(jeu.id));
    }
    let survol = reponse.hovered() || reponse.context_menu_opened();
    let reponse = reponse.on_hover_cursor(egui::CursorIcon::PointingHand);
    let reponse = if survol && !reponse.context_menu_opened() {
        reponse.on_hover_ui_at_pointer(|ui| infobulle_jeu(ui, jeu))
    } else {
        reponse
    };
    reponse.context_menu(|ui| menu_jeu(ui, jeu, actions));
}

fn titre_section(ui: &mut egui::Ui, texte: &str) {
    ui.label(RichText::new(texte).size(18.0).family(theme::gras()));
    ui.add_space(6.0);
}

/// Petite étiquette arrondie cliquable (filtres, tags).
fn puce(ui: &mut egui::Ui, texte: &str, actif: bool) -> egui::Response {
    let p = theme::p();
    let (fond, encre) = if actif {
        (p.accent, p.sur_accent())
    } else {
        (p.bg_btn, p.fg)
    };
    ui.add(
        egui::Button::new(RichText::new(texte).color(encre))
            .fill(fond)
            .corner_radius(CornerRadius::same(14)),
    )
}

fn carte(ui: &mut egui::Ui, titre: &str, contenu: impl FnOnce(&mut egui::Ui)) {
    let p = theme::p();
    egui::Frame::new()
        .fill(p.bg_menu)
        .stroke(Stroke::new(1.0, p.bordure))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.label(RichText::new(titre).size(17.0).family(theme::gras()));
                ui.add_space(4.0);
                contenu(ui);
            });
        });
    ui.add_space(14.0);
}

impl eframe::App for App {
    /// Tourne aussi quand la fenêtre est réduite : c'est ici qu'on suit les
    /// parties en cours et qu'on fait revenir le launcher à la fin.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(t) = self.reduire_a {
            if Instant::now() >= t {
                self.reduire_a = None;
                if self.parties.is_empty() {
                    // Rien à suivre (lancement raté entre-temps) : comportement d'origine.
                    if self.config.fermer_apres_lancement {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                } else {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
            } else {
                ctx.request_repaint_after(t - Instant::now());
            }
        }
        self.suivre_parties(ctx);
        self.suivre_manette(ctx);
        if self.sauvegarde_differee && !ctx.input(|i| i.pointer.any_down()) {
            self.sauver();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let p = theme::p();

        self.icones.recevoir(&ctx);
        self.suivre_scan();
        self.suivre_suppressions();
        self.suivre_visuels();
        self.suivre_lot();
        self.suivre_maj();
        self.raccourcis_clavier(&ctx);

        let mut actions = Vec::new();
        if self.mode_tv && self.dialogue.is_none() {
            self.clavier_tv(&ctx);
        }
        self.traiter_commandes(&ctx, &mut actions);

        if self.mode_tv {
            self.vue_tv(ui, &mut actions);
        } else {
            egui::Panel::top("entete")
                .frame(egui::Frame::new().fill(p.bg_main))
                .show(ui, |ui| self.entete(ui, &mut actions));

            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(p.bg_main))
                .show(ui, |ui| match self.vue {
                    Vue::Bibliotheque => self.bibliotheque(ui, &mut actions),
                    Vue::Parametres => self.parametres(ui),
                    Vue::Statistiques => self.statistiques(ui),
                });
        }

        for a in actions {
            self.executer(a, &ctx);
        }

        self.fenetres(&ctx);
        self.afficher_notification(&ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.enregistrer_parties_en_cours();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apercu_des_notes_sur_une_ligne() {
        assert_eq!(apercu_notes("  a\n\n b  c "), "a b c");
        assert_eq!(apercu_notes(""), "");
    }

    #[test]
    fn apercu_des_notes_coupe_les_longs_textes() {
        let long = "é".repeat(APERCU_NOTES_MAX + 10);
        let apercu = apercu_notes(&long);
        assert_eq!(apercu.chars().count(), APERCU_NOTES_MAX + 1);
        assert!(apercu.ends_with('…'));
        assert_eq!(apercu_notes(&"é".repeat(APERCU_NOTES_MAX)).chars().count(), APERCU_NOTES_MAX);
    }
}
