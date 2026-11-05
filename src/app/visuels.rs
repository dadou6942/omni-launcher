//! Jaquettes et bannières : boîte de dialogue d'un jeu, récupération groupée
//! depuis SteamGridDB et réglage de la clé d'API.

use super::*;
use crate::steamgriddb::{self, ResultatJeu, Telecharges};

/// Le lot s'arrête après ce nombre d'échecs d'affilée (clé refusée, réseau…).
const ERREURS_MAX: u32 = 3;

enum MsgEdition {
    Resultats(Result<Vec<ResultatJeu>, String>),
    Images(Result<Telecharges, String>),
}

/// État de la boîte de dialogue d'un jeu.
pub(super) struct EditionVisuels {
    id: u64,
    recherche: String,
    resultats: Vec<ResultatJeu>,
    tache: Option<Receiver<MsgEdition>>,
    erreur: Option<String>,
}

enum MsgLot {
    Progression { fait: usize, total: usize, nom: String },
    Images { id: u64, images: Telecharges },
    Fini { erreur: Option<String> },
}

/// Récupération des images manquantes de toute la bibliothèque.
pub(super) struct LotVisuels {
    reception: Receiver<MsgLot>,
    annule: Arc<AtomicBool>,
    fait: usize,
    total: usize,
    nom: String,
    trouves: usize,
}

/// Ce que l'utilisateur a demandé dans la boîte de dialogue.
enum Choix {
    Fichier(Genre),
    Retirer(Genre),
    Rechercher,
    Telecharger(u64),
    Fermer,
}

fn traiter_lot(
    cle: String,
    jeux: Vec<(u64, String, Vec<Genre>)>,
    dossier: PathBuf,
    annule: Arc<AtomicBool>,
    tx: std::sync::mpsc::Sender<MsgLot>,
    ctx: egui::Context,
) {
    let total = jeux.len();
    let mut echecs = 0;
    let mut erreur = None;
    for (i, (id, nom, genres)) in jeux.into_iter().enumerate() {
        if annule.load(Ordering::Relaxed) || echecs >= ERREURS_MAX {
            break;
        }
        let _ = tx.send(MsgLot::Progression { fait: i, total, nom: nom.clone() });
        ctx.request_repaint();
        let resultat = steamgriddb::rechercher(&cle, &nom).and_then(|r| match r.first() {
            Some(j) => steamgriddb::telecharger_visuels(&cle, j.id, id, &genres, &dossier),
            None => Ok(Telecharges::default()),
        });
        match resultat {
            Ok(images) => {
                echecs = 0;
                erreur = None;
                let _ = tx.send(MsgLot::Images { id, images });
            }
            Err(e) => {
                echecs += 1;
                erreur = Some(e);
            }
        }
    }
    let _ = tx.send(MsgLot::Fini { erreur });
    ctx.request_repaint();
}

impl App {
    /// Remplace (ou retire, avec `None`) une image d'un jeu et supprime
    /// l'ancien fichier. Le jeu a pu disparaître pendant le téléchargement.
    fn changer_visuel(&mut self, id: u64, genre: Genre, nom: Option<String>) {
        let Some(jeu) = self.config.jeu_mut(id) else {
            if let Some(n) = nom {
                let _ = std::fs::remove_file(self.dossiers.jaquettes.join(n));
            }
            return;
        };
        let champ = match genre {
            Genre::Banniere => &mut jeu.banniere,
            Genre::Jaquette | Genre::Icone => &mut jeu.jaquette,
        };
        let ancien = std::mem::replace(champ, nom.clone());
        if let Some(a) = ancien
            && Some(&a) != nom.as_ref()
        {
            let _ = std::fs::remove_file(self.dossiers.jaquettes.join(a));
        }
        self.icones.invalider_genre(id, genre);
    }

    fn appliquer_images(&mut self, id: u64, images: Telecharges) -> usize {
        let mut n = 0;
        for (genre, nom) in [(Genre::Jaquette, images.jaquette), (Genre::Banniere, images.banniere)] {
            if nom.is_some() {
                self.changer_visuel(id, genre, nom);
                n += 1;
            }
        }
        n
    }

    pub(super) fn ouvrir_visuels(&mut self, id: u64) {
        if let Some(nom) = self.jeu(id).map(|j| j.nom.clone()) {
            self.edition_visuels = Some(EditionVisuels {
                id,
                recherche: nom,
                resultats: Vec::new(),
                tache: None,
                erreur: None,
            });
        }
    }

    fn lancer_recherche(&mut self, ctx: &egui::Context) {
        let cle = self.cle_sgdb.clone();
        let Some(e) = &mut self.edition_visuels else { return };
        let terme = e.recherche.trim().to_string();
        if terme.is_empty() || e.tache.is_some() {
            return;
        }
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(MsgEdition::Resultats(steamgriddb::rechercher(&cle, &terme)));
            ctx.request_repaint();
        });
        e.erreur = None;
        e.tache = Some(rx);
    }

    fn lancer_telechargement(&mut self, id_sgdb: u64, ctx: &egui::Context) {
        let cle = self.cle_sgdb.clone();
        let dossier = self.dossiers.jaquettes.clone();
        let Some(e) = &mut self.edition_visuels else { return };
        if e.tache.is_some() {
            return;
        }
        let id = e.id;
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let genres = [Genre::Jaquette, Genre::Banniere];
            let r = steamgriddb::telecharger_visuels(&cle, id_sgdb, id, &genres, &dossier);
            let _ = tx.send(MsgEdition::Images(r));
            ctx.request_repaint();
        });
        e.erreur = None;
        e.tache = Some(rx);
    }

    pub(super) fn suivre_visuels(&mut self) {
        let Some(e) = &mut self.edition_visuels else { return };
        let Some(rx) = &e.tache else { return };
        let message = match rx.try_recv() {
            Ok(m) => m,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                e.tache = None;
                e.erreur = Some("La recherche s'est interrompue.".into());
                return;
            }
        };
        e.tache = None;
        match message {
            MsgEdition::Resultats(Ok(r)) => {
                if r.is_empty() {
                    e.erreur = Some("Aucun jeu trouvé sur SteamGridDB.".into());
                }
                e.resultats = r;
            }
            MsgEdition::Resultats(Err(m)) | MsgEdition::Images(Err(m)) => e.erreur = Some(m),
            MsgEdition::Images(Ok(images)) => {
                let id = e.id;
                if self.appliquer_images(id, images) == 0 {
                    if let Some(e) = &mut self.edition_visuels {
                        e.erreur = Some("Ce jeu n'a pas d'image sur SteamGridDB.".into());
                    }
                } else {
                    self.sauver();
                    self.notifier("Images enregistrées.");
                }
            }
        }
    }

    fn choisir_fichier_visuel(&mut self, id: u64, genre: Genre) {
        let titre = match genre {
            Genre::Banniere => "Choisir une bannière",
            Genre::Jaquette | Genre::Icone => "Choisir une jaquette",
        };
        let Some(fichier) = rfd::FileDialog::new()
            .set_title(titre)
            .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
            .pick_file()
        else {
            return;
        };
        match steamgriddb::importer_image(&fichier, id, genre, &self.dossiers.jaquettes) {
            Ok(nom) => {
                self.changer_visuel(id, genre, Some(nom));
                self.sauver();
            }
            Err(e) => self.notifier_erreur(e),
        }
    }

    pub(super) fn fenetre_visuels(&mut self, ctx: &egui::Context) {
        let Some(id) = self.edition_visuels.as_ref().map(|e| e.id) else {
            return;
        };
        let Some(jeu) = self.jeu(id).cloned() else {
            self.edition_visuels = None;
            return;
        };
        let p = theme::p();
        let a_une_cle = !self.cle_sgdb.is_empty();
        let mut choix = None;
        let jaquette = jeu
            .jaquette
            .as_deref()
            .and_then(|f| self.icones.visuel(id, Genre::Jaquette, f));
        let banniere = jeu
            .banniere
            .as_deref()
            .and_then(|f| self.icones.visuel(id, Genre::Banniere, f));
        let Some(e) = &mut self.edition_visuels else { return };

        let reponse = egui::Modal::new(egui::Id::unique("visuels")).show(ctx, |ui| {
            ui.set_width(520.0f32.min(ctx.content_rect().width() - 60.0));
            ui.heading("Jaquette et bannière");
            ui.label(RichText::new(format!("Pour « {} »", jeu.nom)).color(p.fg_muted));
            ui.add_space(8.0);

            ligne_visuel(ui, "Jaquette", jeu.jaquette.is_some(), jaquette, Genre::Jaquette, &mut choix);
            ui.add_space(6.0);
            ligne_visuel(ui, "Bannière", jeu.banniere.is_some(), banniere, Genre::Banniere, &mut choix);

            ui.separator();
            ui.label(RichText::new("Chercher sur SteamGridDB").strong());
            if !a_une_cle {
                ui.label(
                    RichText::new("Ajoutez votre clé SteamGridDB dans les paramètres pour activer la recherche.")
                        .color(p.fg_muted),
                );
            } else {
                recherche_steamgriddb(ui, e, &mut choix);
            }

            if let Some(m) = &e.erreur {
                ui.label(RichText::new(m.as_str()).color(p.danger));
            }
            ui.add_space(10.0);
            // Fermer pendant un téléchargement laisserait ses fichiers orphelins.
            if ui.add_enabled(e.tache.is_none(), egui::Button::new("Terminé")).clicked() {
                choix = Some(Choix::Fermer);
            }
        });

        if reponse.should_close() && e.tache.is_none() {
            choix = Some(Choix::Fermer);
        }
        match choix {
            Some(Choix::Fichier(genre)) => self.choisir_fichier_visuel(id, genre),
            Some(Choix::Retirer(genre)) => {
                self.changer_visuel(id, genre, None);
                self.sauver();
            }
            Some(Choix::Rechercher) => self.lancer_recherche(ctx),
            Some(Choix::Telecharger(id_sgdb)) => self.lancer_telechargement(id_sgdb, ctx),
            Some(Choix::Fermer) => self.edition_visuels = None,
            None => {}
        }
    }

    pub(super) fn enregistrer_cle(&mut self) {
        match steamgriddb::ecrire_cle(&self.dossiers, &self.cle_sgdb) {
            Ok(()) => self.cle_sgdb = self.cle_sgdb.trim().to_string(),
            Err(e) => self.notifier_erreur(format!("Impossible d'enregistrer la clé : {e}")),
        }
    }

    /// Jeux à compléter, avec les images qui leur manquent.
    fn jeux_incomplets(&self) -> Vec<(u64, String, Vec<Genre>)> {
        self.config
            .jeux
            .iter()
            .filter_map(|j| {
                let manquantes: Vec<Genre> = [
                    j.jaquette.is_none().then_some(Genre::Jaquette),
                    j.banniere.is_none().then_some(Genre::Banniere),
                ]
                .into_iter()
                .flatten()
                .collect();
                (!manquantes.is_empty()).then(|| (j.id, j.nom.clone(), manquantes))
            })
            .collect()
    }

    fn lancer_lot(&mut self, ctx: &egui::Context) {
        let jeux = self.jeux_incomplets();
        if jeux.is_empty() {
            self.notifier("Tous les jeux ont déjà leurs images.");
            return;
        }
        let (tx, reception) = channel();
        let annule = Arc::new(AtomicBool::new(false));
        let total = jeux.len();
        let (cle, dossier) = (self.cle_sgdb.clone(), self.dossiers.jaquettes.clone());
        let (annule_thread, ctx) = (Arc::clone(&annule), ctx.clone());
        std::thread::spawn(move || traiter_lot(cle, jeux, dossier, annule_thread, tx, ctx));
        self.lot_visuels = Some(LotVisuels {
            reception,
            annule,
            fait: 0,
            total,
            nom: String::new(),
            trouves: 0,
        });
    }

    pub(super) fn suivre_lot(&mut self) {
        loop {
            let Some(lot) = &mut self.lot_visuels else { return };
            let message = match lot.reception.try_recv() {
                Ok(m) => m,
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
                // Thread interrompu : ne pas laisser la fenêtre bloquée.
                Err(std::sync::mpsc::TryRecvError::Disconnected) => MsgLot::Fini {
                    erreur: Some("la récupération s'est interrompue.".into()),
                },
            };
            match message {
                MsgLot::Progression { fait, total, nom } => {
                    lot.fait = fait;
                    lot.total = total;
                    lot.nom = nom;
                }
                MsgLot::Images { id, images } => {
                    if self.appliquer_images(id, images) > 0
                        && let Some(lot) = &mut self.lot_visuels
                    {
                        lot.trouves += 1;
                    }
                }
                MsgLot::Fini { erreur } => {
                    let trouves = lot.trouves;
                    self.lot_visuels = None;
                    self.sauver();
                    match erreur {
                        Some(e) => self.notifier_erreur(format!(
                            "Récupération interrompue après {trouves} jeu(x) : {e}"
                        )),
                        None => self.notifier(format!("Images trouvées pour {trouves} jeu(x).")),
                    }
                    return;
                }
            }
        }
    }

    pub(super) fn fenetre_lot(&mut self, ctx: &egui::Context) {
        let Some(lot) = &self.lot_visuels else { return };
        let p = theme::p();
        let annuler = egui::Modal::new(egui::Id::unique("lot_visuels")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.heading("Récupération des images…");
            ui.add_space(8.0);
            let progression = lot.fait as f32 / lot.total.max(1) as f32;
            ui.add(
                egui::ProgressBar::new(progression)
                    .show_percentage()
                    .fill(p.accent.linear_multiply(0.6)),
            );
            ui.label(RichText::new(&lot.nom).color(p.fg_muted));
            ui.add_space(8.0);
            let annule = lot.annule.load(Ordering::Relaxed);
            ui.add_enabled(!annule, egui::Button::new("Annuler")).clicked()
        });
        if annuler.inner {
            lot.annule.store(true, Ordering::Relaxed);
        }
    }

    pub(super) fn carte_visuels(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let p = theme::p();
        ui.label(
            RichText::new(
                "SteamGridDB fournit des jaquettes et des bannières. Sa clé d'API est gratuite. \
                 Elle reste sur ce PC, hors des sauvegardes et des exports.",
            )
            .color(p.fg_muted),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label("Clé d'API");
            let champ = ui.add(
                egui::TextEdit::singleline(&mut self.cle_sgdb)
                    .password(true)
                    .desired_width(300.0),
            );
            if champ.lost_focus() {
                self.enregistrer_cle();
            }
            if ui.button("Obtenir une clé").clicked()
                && let Err(e) = platform::ouvrir_url(steamgriddb::PAGE_CLE_API)
            {
                self.notifier_erreur(e);
            }
        });
        let incomplets = self.jeux_incomplets().len();
        let bouton = egui::Button::new(format!("Récupérer les images manquantes ({incomplets})"));
        if ui
            .add_enabled(!self.cle_sgdb.trim().is_empty() && incomplets > 0, bouton)
            .on_hover_text("Prend le premier résultat de chaque jeu. Corrigez au cas par cas : clic droit ➔ Jaquette et bannière…")
            .clicked()
        {
            self.enregistrer_cle();
            self.lancer_lot(ctx);
        }
    }
}

fn ligne_visuel(
    ui: &mut egui::Ui,
    titre: &str,
    definie: bool,
    texture: Option<egui::TextureHandle>,
    genre: Genre,
    choix: &mut Option<Choix>,
) {
    let p = theme::p();
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(titre).strong());
            ui.horizontal(|ui| {
                if ui.button("Choisir une image…").clicked() {
                    *choix = Some(Choix::Fichier(genre));
                }
                if definie && ui.button("Retirer").clicked() {
                    *choix = Some(Choix::Retirer(genre));
                }
            });
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| match texture {
            Some(t) => {
                ui.add(egui::Image::new(&t).max_size(Vec2::new(200.0, 80.0)));
            }
            None => {
                ui.label(RichText::new("Aucune image").italics().color(p.fg_muted));
            }
        });
    });
}

fn recherche_steamgriddb(ui: &mut egui::Ui, e: &mut EditionVisuels, choix: &mut Option<Choix>) {
    let p = theme::p();
    let occupe = e.tache.is_some();
    ui.horizontal(|ui| {
        let champ = ui.add(egui::TextEdit::singleline(&mut e.recherche).desired_width(320.0));
        let entree = champ.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        if ui.add_enabled(!occupe, egui::Button::new("Rechercher")).clicked() || entree {
            *choix = Some(Choix::Rechercher);
        }
        if occupe {
            ui.spinner();
        }
    });
    if !e.resultats.is_empty() {
        ui.label(RichText::new("Choisissez le bon jeu :").color(p.fg_muted));
    }
    for r in &e.resultats {
        if ui.add_enabled(!occupe, egui::Button::new(&r.nom)).clicked() {
            *choix = Some(Choix::Telecharger(r.id));
        }
    }
}
