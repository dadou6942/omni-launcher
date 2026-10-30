//! Interface principale : bibliothèque, recherche, scan, paramètres.

use crate::config::{
    Config, Dossiers, Jeu, TAILLE_TUILE_MAX, TAILLE_TUILE_MIN, Tri, cle_chemin, maintenant,
};
use crate::icons::{self, ChargeurIcones};
use crate::platform;
use crate::scan::{self, Candidat};
use crate::theme::{self, ACCENT, BG_BTN, BG_TUILE_HOVER, BORDURE, DANGER, FG, FG_MUTED};
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

/// Action différée exécutée après la fermeture d'une boîte de dialogue.
type ActionDialogue = Box<dyn FnOnce(&mut App, &egui::Context)>;

#[derive(PartialEq, Eq, Clone, Copy)]
enum Vue {
    Bibliotheque,
    Parametres,
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

pub struct App {
    dossiers: Dossiers,
    config: Config,
    icones: ChargeurIcones,
    vue: Vue,
    recherche: String,
    focus_recherche: bool,
    renommage: Option<Renommage>,
    dialogue: Option<Dialogue>,
    scan: Option<ScanEnCours>,
    revue: Option<Revue>,
    manquants: HashSet<u64>,
    notification: Option<Notification>,
    suppressions: Vec<Receiver<Result<String, String>>>,
    fermer_a: Option<Instant>,
}

impl App {
    pub fn nouveau(ctx: &egui::Context) -> Self {
        let dossiers = Dossiers::detecter();
        let config = Config::charger(&dossiers.config);
        let icones = ChargeurIcones::nouveau(ctx, &dossiers);
        let mut app = Self {
            dossiers,
            config,
            icones,
            vue: Vue::Bibliotheque,
            recherche: String::new(),
            focus_recherche: false,
            renommage: None,
            dialogue: None,
            scan: None,
            revue: None,
            manquants: HashSet::new(),
            notification: None,
            suppressions: Vec::new(),
            fermer_a: None,
        };
        app.verifier_manquants();
        app
    }

    // ------------------------------------------------------------------
    // Utilitaires
    // ------------------------------------------------------------------

    fn sauver(&mut self) {
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

    /// Identifiants des jeux visibles, filtrés et triés, séparés en
    /// (favoris, autres).
    fn listes_visibles(&self) -> (Vec<u64>, Vec<u64>) {
        let termes: Vec<String> = self
            .recherche
            .split_whitespace()
            .map(|t| t.to_lowercase())
            .collect();

        let mut jeux: Vec<&Jeu> = self
            .config
            .jeux
            .iter()
            .filter(|j| {
                let nom = j.nom.to_lowercase();
                termes.iter().all(|t| nom.contains(t.as_str()))
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
        }

        let (fav, autres): (Vec<&Jeu>, Vec<&Jeu>) = jeux.into_iter().partition(|j| j.favori);
        (
            fav.iter().map(|j| j.id).collect(),
            autres.iter().map(|j| j.id).collect(),
        )
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
        match platform::lancer(&jeu.exe, &jeu.arguments) {
            Ok(()) => {
                if let Some(j) = self.config.jeu_mut(id) {
                    j.dernier_lancement = Some(maintenant());
                }
                self.sauver();
                self.notifier(format!("Lancement de « {} »…", jeu.nom));
                if self.config.fermer_apres_lancement {
                    self.fermer_a = Some(Instant::now() + Duration::from_millis(1200));
                    ctx.request_repaint_after(Duration::from_millis(1200));
                }
            }
            Err(e) => self.notifier_erreur(format!("Impossible de lancer « {} » : {e}", jeu.nom)),
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
    // Interface
    // ------------------------------------------------------------------

    fn raccourcis_clavier(&mut self, ctx: &egui::Context) {
        let (ctrl_f, deposes) = ctx.input(|i| {
            (
                i.modifiers.command && i.key_pressed(Key::F),
                i.raw
                    .dropped_files
                    .iter()
                    .map(|f| f.path().to_path_buf())
                    .filter(|p| !p.as_os_str().is_empty())
                    .collect::<Vec<_>>(),
            )
        });
        if ctrl_f {
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

    fn entete(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let ctx = ui.ctx().clone();
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.add_space(14.0);
            match self.vue {
                Vue::Bibliotheque => {
                    ui.heading("Bibliothèque");
                    ui.label(
                        RichText::new(format!("{} jeux", self.config.jeux.len())).color(FG_MUTED),
                    );
                }
                Vue::Parametres => {
                    if ui.button("⬅  Retour à la bibliothèque").clicked() {
                        self.vue = Vue::Bibliotheque;
                    }
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(14.0);
                if self.vue == Vue::Bibliotheque {
                    if ui.button("⚙  Paramètres").clicked() {
                        self.vue = Vue::Parametres;
                    }
                    let scan_btn = ui.add_enabled(
                        self.scan.is_none(),
                        egui::Button::new("🗁  Scanner un dossier"),
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
                                RichText::new("+  Ajouter un jeu").color(Color32::BLACK),
                            )
                            .fill(ACCENT),
                        )
                        .clicked()
                    {
                        self.choisir_exe();
                    }

                    ui.add_space(8.0);
                    egui::ComboBox::from_id_salt("tri")
                        .selected_text(match self.config.tri {
                            Tri::Nom => "Tri : nom",
                            Tri::DernierLancement => "Tri : récents",
                            Tri::DateAjout => "Tri : ajout",
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

    fn bibliotheque(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let ctx = ui.ctx().clone();
        if self.config.jeux.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.25);
                ui.label(RichText::new("Aucun jeu dans votre bibliothèque").size(22.0).color(FG));
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Ajoutez un exécutable, scannez votre dossier de jeux, ou glissez-déposez-les ici.")
                        .color(FG_MUTED),
                );
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    let largeur = 420.0;
                    ui.add_space(((ui.available_width() - largeur) / 2.0).max(0.0));
                    if ui
                        .add(egui::Button::new(RichText::new("+  Ajouter un jeu").color(Color32::BLACK)).fill(ACCENT))
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
        let recherche_active = !self.recherche.trim().is_empty();

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(4.0);
            egui::Frame::new().inner_margin(egui::Margin::symmetric(24, 0)).show(ui, |ui| {
                if recherche_active && favoris.is_empty() && autres.is_empty() {
                    ui.add_space(40.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new(format!("Aucun jeu ne correspond à « {} ».", self.recherche.trim())).color(FG_MUTED));
                    });
                    return;
                }

                if !recherche_active || !favoris.is_empty() {
                    titre_section(ui, "Vos favoris ⭐");
                    if favoris.is_empty() {
                        ui.label(
                            RichText::new("Aucun favori pour le moment. Clic droit sur un jeu ➔ Ajouter aux favoris.")
                                .italics()
                                .color(FG_MUTED),
                        );
                        ui.add_space(16.0);
                    } else {
                        self.grille(ui, &favoris, actions);
                    }
                }

                if !autres.is_empty() {
                    ui.add_space(12.0);
                    titre_section(ui, if recherche_active { "Résultats" } else { "Autres jeux" });
                    self.grille(ui, &autres, actions);
                }
                ui.add_space(24.0);
            });
        });
    }

    fn grille(&mut self, ui: &mut egui::Ui, ids: &[u64], actions: &mut Vec<Action>) {
        let cote = self.config.taille_tuile;
        let taille = Vec2::new(cote + 24.0, cote + 56.0);
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

    fn tuile(&mut self, ui: &mut egui::Ui, id: u64, taille: Vec2, actions: &mut Vec<Action>) {
        let Some(jeu) = self.jeu(id).cloned() else {
            return;
        };
        let manquant = self.manquants.contains(&id);
        let cote = self.config.taille_tuile;

        let (rect, reponse) = ui.allocate_exact_size(taille, Sense::click());
        if !ui.is_rect_visible(rect) {
            return; // Tuile hors écran : rien à dessiner ni à charger.
        }
        let survol = reponse.hovered() || reponse.context_menu_opened();
        let peintre = ui.painter_at(rect.expand(2.0));

        if survol {
            peintre.rect_filled(rect, CornerRadius::same(10), BG_TUILE_HOVER);
        }

        // Icône
        let zone_icone =
            egui::Rect::from_min_size(rect.min + Vec2::new(12.0, 10.0), Vec2::splat(cote));
        let texture = self
            .icones
            .texture(id, &jeu.exe, jeu.icone_perso.as_deref());
        match texture {
            Some(t) => {
                let ts = t.size_vec2();
                let echelle = (cote / ts.x).min(cote / ts.y);
                let r = egui::Rect::from_center_size(zone_icone.center(), ts * echelle);
                let mut img = egui::Image::new(&t).corner_radius(CornerRadius::same(8));
                if manquant {
                    img = img.tint(Color32::from_gray(90));
                }
                img.paint_at(ui, r);
            }
            None => {
                // Icône provisoire : initiale du jeu.
                peintre.rect_filled(zone_icone.shrink(6.0), CornerRadius::same(14), BG_BTN);
                peintre.rect_stroke(
                    zone_icone.shrink(6.0),
                    CornerRadius::same(14),
                    Stroke::new(1.5, BORDURE),
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
                    zone_icone.center(),
                    Align2::CENTER_CENTER,
                    initiale,
                    FontId::new(cote * 0.42, theme::gras()),
                    FG_MUTED,
                );
            }
        }

        if manquant {
            peintre.text(
                zone_icone.center_bottom() - Vec2::new(0.0, 8.0),
                Align2::CENTER_BOTTOM,
                "Introuvable",
                FontId::new(13.0, theme::gras()),
                DANGER,
            );
        }

        // Nom (ou champ de renommage)
        let zone_nom = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 4.0, zone_icone.bottom() + 6.0),
            egui::pos2(rect.right() - 4.0, rect.bottom()),
        );

        let en_renommage = self.renommage.as_ref().is_some_and(|r| r.id == id);
        if en_renommage {
            let r = self.renommage.as_mut().unwrap();
            let edit = ui.put(
                egui::Rect::from_min_size(zone_nom.min, Vec2::new(zone_nom.width(), 28.0)),
                egui::TextEdit::singleline(&mut r.texte)
                    .align(Align2::CENTER_CENTER)
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
        } else {
            let mut job = egui::text::LayoutJob::simple(
                jeu.nom.clone(),
                FontId::new(14.0, theme::gras()),
                if manquant { FG_MUTED } else { FG },
                zone_nom.width(),
            );
            job.halign = egui::Align::Center;
            job.wrap.max_rows = 2;
            job.wrap.break_anywhere = false;
            job.wrap.overflow_character = Some('…');
            let galley = ui.painter().layout_job(job);
            peintre.galley(egui::pos2(zone_nom.center().x, zone_nom.top()), galley, FG);
        }

        // Interactions
        if reponse.double_clicked() {
            actions.push(Action::Lancer(id));
        }
        let reponse = reponse.on_hover_cursor(egui::CursorIcon::PointingHand);
        let reponse = if survol && !reponse.context_menu_opened() {
            reponse.on_hover_text_at_pointer(
                RichText::new(jeu.exe.display().to_string())
                    .small()
                    .color(FG_MUTED),
            )
        } else {
            reponse
        };

        reponse.context_menu(|ui| {
            ui.set_min_width(230.0);
            ui.label(RichText::new(&jeu.nom).strong().color(ACCENT));
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
            ui.separator();
            if ui.button("✏  Renommer").clicked() {
                actions.push(Action::DebuterRenommage(id));
                ui.close();
            }
            if ui.button("🖼  Modifier l'icône…").clicked() {
                actions.push(Action::ChangerIcone(id));
                ui.close();
            }
            if jeu.icone_perso.is_some() && ui.button("↺  Rétablir l'icône d'origine").clicked()
            {
                actions.push(Action::RetablirIcone(id));
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
                .button(RichText::new("🗑  Supprimer le jeu du disque…").color(DANGER))
                .clicked()
            {
                actions.push(Action::DemanderSuppression(id));
                ui.close();
            }
        });
    }

    fn parametres(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.set_max_width(640.0);
                ui.add_space(10.0);
                ui.label(RichText::new("Configuration du launcher").size(26.0).family(theme::gras()));
                ui.add_space(24.0);

                carte(ui, "Affichage", |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().slider_width = 260.0;
                        ui.label("Taille des icônes");
                        let r = ui.add(
                            egui::Slider::new(&mut self.config.taille_tuile, TAILLE_TUILE_MIN..=TAILLE_TUILE_MAX)
                                .step_by(5.0)
                                .suffix(" px"),
                        );
                        if r.drag_stopped() || (r.changed() && !r.dragged()) {
                            self.sauver();
                        }
                    });
                    if ui
                        .checkbox(&mut self.config.fermer_apres_lancement, "Fermer le launcher après le lancement d'un jeu")
                        .changed()
                    {
                        self.sauver();
                    }
                });

                carte(ui, "Bibliothèque", |ui| {
                    let favoris = self.config.jeux.iter().filter(|j| j.favori).count();
                    ui.label(format!("{} jeux, dont {} favoris.", self.config.jeux.len(), favoris));
                    if let Some(d) = &self.config.dernier_dossier_scan {
                        ui.label(RichText::new(format!("Dernier dossier scanné : {}", d.display())).color(FG_MUTED));
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui.add_enabled(self.scan.is_none(), egui::Button::new("🗁  Scanner un dossier…")).clicked() {
                            self.choisir_dossier_scan(&ctx);
                        }
                        if let Some(d) = self.config.dernier_dossier_scan.clone()
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
                            self.verifier_manquants();
                            let n = self.manquants.len();
                            self.notifier(match n {
                                0 => "Tous les jeux sont présents.".to_string(),
                                n => format!("{n} jeu(x) introuvable(s)."),
                            });
                        }
                        let n = self.manquants.len();
                        if ui
                            .add_enabled(n > 0, egui::Button::new(format!("Retirer les introuvables ({n})")))
                            .clicked()
                        {
                            self.dialogue = Some(Dialogue::RetirerIntrouvables);
                        }
                    });
                });

                carte(ui, "Données", |ui| {
                    ui.label(RichText::new("Configuration, icônes personnalisées et cache :").color(FG_MUTED));
                    ui.label(RichText::new(self.dossiers.racine.display().to_string()).color(ACCENT).strong());
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("🗁  Ouvrir le dossier").clicked() {
                            let racine = self.dossiers.racine.clone();
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
                        "Ctrl+F pour rechercher, Entrée lance le premier résultat, Échap efface.",
                        "Glissez-déposez des .exe ou un dossier de jeux directement dans la fenêtre.",
                        "« Supprimer le jeu du disque » envoie son dossier à la corbeille : c'est récupérable.",
                    ] {
                        ui.label(RichText::new(format!("•  {astuce}")).color(FG_MUTED));
                    }
                });
            });
        });
    }

    fn fenetres(&mut self, ctx: &egui::Context) {
        self.fenetre_scan(ctx);
        self.fenetre_revue(ctx);
        self.fenetre_dialogue(ctx);
    }

    fn fenetre_scan(&mut self, ctx: &egui::Context) {
        let Some(scan) = &self.scan else { return };
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
                    .fill(ACCENT.linear_multiply(0.6)),
            );
            ui.label(RichText::new(&scan.nom).color(FG_MUTED));
            ui.add_space(8.0);
            ui.button("Annuler").clicked()
        });
        if annuler.inner {
            scan.annule.store(true, Ordering::Relaxed);
        }
    }

    fn fenetre_revue(&mut self, ctx: &egui::Context) {
        let Some(revue) = &mut self.revue else { return };
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
                .color(FG_MUTED),
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
                            ui.label(RichText::new(&c.groupe).strong().color(ACCENT));
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
                                ui.label(RichText::new(rel).small().color(FG_MUTED))
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
                            RichText::new(format!("Importer {n} jeu(x)")).color(Color32::BLACK),
                        )
                        .fill(ACCENT),
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
        let mut fermer = false;
        let mut action: Option<ActionDialogue> = None;

        let nom = |id: &u64, jeux: &[Jeu]| {
            jeux.iter()
                .find(|j| j.id == *id)
                .map(|j| j.nom.clone())
                .unwrap_or_default()
        };
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
                            .color(FG_MUTED),
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
                    ui.heading(RichText::new("Supprimer le jeu du disque").color(DANGER));
                    ui.label(format!(
                        "Le dossier suivant de « {} » va être envoyé à la corbeille :",
                        nom(&id, jeux)
                    ));
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(cible.display().to_string())
                            .color(ACCENT)
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Vous pourrez le restaurer depuis la corbeille de Windows.")
                            .color(FG_MUTED),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Mettre à la corbeille").color(Color32::BLACK),
                                )
                                .fill(DANGER),
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
                    ui.label(RichText::new(raison.as_str()).color(DANGER));
                    ui.label(
                        RichText::new("Vous pouvez le supprimer vous-même depuis l'Explorateur.")
                            .color(FG_MUTED),
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
                        ui.label(RichText::new(j.exe.display().to_string()).color(FG_MUTED));
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
                                ui.label(RichText::new(format!("•  {n}")).color(FG_MUTED));
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
                        .color(FG_MUTED),
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
            }
        });

        if reponse.should_close() {
            fermer = true;
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
                    .fill(theme::BG_MENU)
                    .stroke(Stroke::new(
                        1.0,
                        if n.erreur {
                            DANGER
                        } else {
                            ACCENT.linear_multiply(0.5)
                        },
                    ))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(16, 10))
                    .show(ui, |ui| {
                        ui.label(RichText::new(&n.texte).color(if n.erreur { DANGER } else { FG }));
                    });
            });
    }
}

fn titre_section(ui: &mut egui::Ui, texte: &str) {
    ui.label(RichText::new(texte).size(18.0).family(theme::gras()));
    ui.add_space(6.0);
}

fn carte(ui: &mut egui::Ui, titre: &str, contenu: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(theme::BG_MENU)
        .stroke(Stroke::new(1.0, BORDURE))
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
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if let Some(t) = self.fermer_a {
            if Instant::now() >= t {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            ctx.request_repaint_after(t - Instant::now());
        }

        self.icones.recevoir(&ctx);
        self.suivre_scan();
        self.suivre_suppressions();
        self.raccourcis_clavier(&ctx);

        let mut actions = Vec::new();

        egui::Panel::top("entete")
            .frame(egui::Frame::new().fill(theme::BG_MAIN))
            .show(ui, |ui| self.entete(ui, &mut actions));

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::BG_MAIN))
            .show(ui, |ui| match self.vue {
                Vue::Bibliotheque => self.bibliotheque(ui, &mut actions),
                Vue::Parametres => self.parametres(ui),
            });

        for a in actions {
            self.executer(a, &ctx);
        }

        self.fenetres(&ctx);
        self.afficher_notification(&ctx);
    }
}
