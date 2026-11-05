//! Configuration persistante : un seul fichier JSON qui remplace l'ancien
//! dossier de raccourcis `.lnk`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const TAILLE_TUILE_MIN: f32 = 90.0;
pub const TAILLE_TUILE_MAX: f32 = 220.0;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Jeu {
    pub id: u64,
    pub nom: String,
    pub exe: PathBuf,
    /// Arguments de lancement optionnels.
    #[serde(default)]
    pub arguments: String,
    #[serde(default)]
    pub favori: bool,
    /// Nom du fichier d'icône personnalisée dans le dossier `icones/`.
    #[serde(default)]
    pub icone_perso: Option<String>,
    /// Dossier à supprimer quand on supprime le jeu du disque
    /// (dossier racine du jeu détecté au scan, sinon dossier de l'exe).
    #[serde(default)]
    pub cible_suppression: Option<PathBuf>,
    #[serde(default)]
    pub ajoute_le: u64,
    #[serde(default)]
    pub dernier_lancement: Option<u64>,
    /// Temps de jeu cumulé, en secondes.
    #[serde(default)]
    pub temps_jeu: u64,
    #[serde(default)]
    pub nb_lancements: u32,
    /// Historique des parties mesurées (les plus récentes à la fin).
    #[serde(default)]
    pub sessions: Vec<Session>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub statut: Statut,
    /// Notes libres de l'utilisateur.
    #[serde(default)]
    pub notes: String,
    /// Nom du fichier de jaquette (portrait) dans le dossier `jaquettes/`.
    #[serde(default)]
    pub jaquette: Option<String>,
    /// Nom du fichier de bannière (large) dans le dossier `jaquettes/`.
    #[serde(default)]
    pub banniere: Option<String>,
}

/// Où en est l'utilisateur avec un jeu.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Statut {
    #[default]
    Aucun,
    AJouer,
    EnCours,
    Termine,
    Abandonne,
}

impl Statut {
    pub const TOUS: [Statut; 5] = [
        Statut::Aucun,
        Statut::AJouer,
        Statut::EnCours,
        Statut::Termine,
        Statut::Abandonne,
    ];

    pub fn libelle(self) -> &'static str {
        match self {
            Statut::Aucun => "Aucun statut",
            Statut::AJouer => "À jouer",
            Statut::EnCours => "En cours",
            Statut::Termine => "Terminé",
            Statut::Abandonne => "Abandonné",
        }
    }
}

/// Une partie mesurée : début (horodatage Unix) et durée en secondes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub debut: u64,
    pub duree: u64,
}

/// Au-delà, les plus anciennes sessions sont oubliées (le cumul reste juste).
const SESSIONS_MAX: usize = 1000;

impl Jeu {
    pub fn nouveau(id: u64, nom: String, exe: PathBuf, cible_suppression: Option<PathBuf>) -> Self {
        Self {
            id,
            nom,
            exe,
            arguments: String::new(),
            favori: false,
            icone_perso: None,
            cible_suppression,
            ajoute_le: maintenant(),
            dernier_lancement: None,
            temps_jeu: 0,
            nb_lancements: 0,
            sessions: Vec::new(),
            tags: Vec::new(),
            statut: Statut::Aucun,
            notes: String::new(),
            jaquette: None,
            banniere: None,
        }
    }

    pub fn ajouter_session(&mut self, debut: u64, duree: u64) {
        self.temps_jeu += duree;
        self.sessions.push(Session { debut, duree });
        if self.sessions.len() > SESSIONS_MAX {
            let trop = self.sessions.len() - SESSIONS_MAX;
            self.sessions.drain(..trop);
        }
    }

    pub fn a_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t.eq_ignore_ascii_case(tag))
    }

    /// Ajoute un tag (sans doublon, casse ignorée). Renvoie `false` si rien n'a changé.
    pub fn ajouter_tag(&mut self, tag: &str) -> bool {
        let tag = tag.trim();
        if tag.is_empty() || self.a_tag(tag) {
            return false;
        }
        self.tags.push(tag.to_string());
        self.tags.sort_by_key(|t| t.to_lowercase());
        true
    }

    pub fn retirer_tag(&mut self, tag: &str) {
        self.tags.retain(|t| !t.eq_ignore_ascii_case(tag));
    }

    /// Dossier dans lequel on cherche les processus du jeu pour mesurer
    /// le temps de jeu.
    pub fn dossier_jeu(&self) -> Option<PathBuf> {
        self.dossier_a_supprimer()
    }

    /// Dossier qui serait envoyé à la corbeille lors d'une suppression.
    pub fn dossier_a_supprimer(&self) -> Option<PathBuf> {
        self.cible_suppression
            .clone()
            .or_else(|| self.exe.parent().map(Path::to_path_buf))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Tri {
    #[default]
    Nom,
    DernierLancement,
    DateAjout,
    TempsDeJeu,
}

/// Présentation de la bibliothèque.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Affichage {
    #[default]
    Grille,
    Jaquettes,
    Liste,
}

pub const VERSION_CONFIG: u32 = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub taille_tuile: f32,
    /// À la fin d'une partie : fermer le launcher (sinon il réapparaît).
    pub fermer_apres_lancement: bool,
    pub tri: Tri,
    pub affichage: Affichage,
    /// Affiche la section « Joués récemment » en tête de la bibliothèque.
    pub afficher_recents: bool,
    /// Cherche une nouvelle version sur GitHub au démarrage.
    pub verifier_maj: bool,
    pub dernier_dossier_scan: Option<PathBuf>,
    pub theme: Theme,
    /// Couleur d'accent choisie par l'utilisateur, à la place de celle du thème.
    pub accent_perso: Option<[u8; 3]>,
    pub prochain_id: u64,
    pub jeux: Vec<Jeu>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Omni,
    OledNoir,
    Nord,
    Dracula,
    Catppuccin,
    Synthwave,
    Clair,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: VERSION_CONFIG,
            taille_tuile: 140.0,
            fermer_apres_lancement: true,
            tri: Tri::Nom,
            affichage: Affichage::Grille,
            afficher_recents: true,
            verifier_maj: true,
            dernier_dossier_scan: None,
            theme: Theme::Omni,
            accent_perso: None,
            prochain_id: 1,
            jeux: Vec::new(),
        }
    }
}

impl Config {
    pub fn charger(chemin: &Path) -> Self {
        match fs::read_to_string(chemin) {
            Ok(txt) => Self::depuis_json(&txt).unwrap_or_else(|_| {
                // Fichier illisible : on le met de côté au lieu de l'écraser.
                let _ = fs::copy(chemin, chemin.with_extension("json.bak"));
                Config::default()
            }),
            Err(_) => Config::default(),
        }
    }

    /// Lit une configuration (fichier principal, sauvegarde ou export) et la
    /// remet d'aplomb : les champs absents des anciennes versions prennent
    /// leur valeur par défaut.
    pub fn depuis_json(txt: &str) -> Result<Self, serde_json::Error> {
        let mut config: Config = serde_json::from_str(txt)?;
        config.version = VERSION_CONFIG;
        config.taille_tuile = config
            .taille_tuile
            .clamp(TAILLE_TUILE_MIN, TAILLE_TUILE_MAX);
        // Sécurité si le fichier a été édité à la main.
        let max_id = config.jeux.iter().map(|j| j.id).max().unwrap_or(0);
        config.prochain_id = config.prochain_id.max(max_id + 1);
        // Un fichier importé ne doit pas pouvoir désigner un fichier ailleurs
        // sur le disque : ces noms servent aussi à supprimer des images.
        for j in &mut config.jeux {
            for nom in [&mut j.icone_perso, &mut j.jaquette, &mut j.banniere] {
                if nom.as_deref().is_some_and(|n| !nom_de_fichier_simple(n)) {
                    *nom = None;
                }
            }
        }
        Ok(config)
    }

    /// Ajoute les jeux d'une autre configuration absents de celle-ci
    /// (comparaison par exécutable). Renvoie le nombre de jeux ajoutés.
    pub fn fusionner(&mut self, autre: Config) -> usize {
        let mut n = 0;
        for mut jeu in autre.jeux {
            if self.contient_exe(&jeu.exe) {
                continue;
            }
            jeu.id = self.nouvel_id();
            self.jeux.push(jeu);
            n += 1;
        }
        n
    }

    /// Tous les tags utilisés, triés, avec le nombre de jeux de chacun.
    pub fn tags(&self) -> Vec<(String, usize)> {
        let mut tags: Vec<(String, usize)> = Vec::new();
        for t in self.jeux.iter().flat_map(|j| &j.tags) {
            match tags.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(t)) {
                Some((_, c)) => *c += 1,
                None => tags.push((t.clone(), 1)),
            }
        }
        tags.sort_by_key(|(n, _)| n.to_lowercase());
        tags
    }

    /// Statuts utilisés (sauf « aucun »), avec le nombre de jeux de chacun.
    pub fn statuts(&self) -> Vec<(Statut, usize)> {
        Statut::TOUS
            .into_iter()
            .filter(|s| *s != Statut::Aucun)
            .map(|s| (s, self.jeux.iter().filter(|j| j.statut == s).count()))
            .filter(|(_, n)| *n > 0)
            .collect()
    }

    /// Les `max` derniers jeux lancés, du plus récent au plus ancien.
    pub fn recents(&self, max: usize) -> Vec<u64> {
        let mut lances: Vec<(u64, u64)> = self
            .jeux
            .iter()
            .filter_map(|j| j.dernier_lancement.map(|t| (t, j.id)))
            .collect();
        lances.sort_by(|a, b| b.cmp(a));
        lances.into_iter().take(max).map(|(_, id)| id).collect()
    }

    pub fn renommer_tag(&mut self, ancien: &str, nouveau: &str) {
        let nouveau = nouveau.trim();
        if nouveau.is_empty() {
            return;
        }
        for j in &mut self.jeux {
            if j.a_tag(ancien) {
                j.retirer_tag(ancien);
                j.ajouter_tag(nouveau);
            }
        }
    }

    pub fn supprimer_tag(&mut self, tag: &str) {
        for j in &mut self.jeux {
            j.retirer_tag(tag);
        }
    }

    /// Écriture atomique : fichier temporaire puis renommage, pour ne jamais
    /// laisser un JSON à moitié écrit en cas de coupure.
    pub fn sauvegarder(&self, chemin: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        let tmp = chemin.with_extension("json.tmp");
        {
            let mut f = fs::File::create(&tmp)?;
            f.write_all(json.as_bytes())?;
            f.sync_all()?;
        }
        fs::rename(&tmp, chemin)
    }

    pub fn nouvel_id(&mut self) -> u64 {
        let id = self.prochain_id;
        self.prochain_id += 1;
        id
    }

    pub fn contient_exe(&self, exe: &Path) -> bool {
        let cle = cle_chemin(exe);
        self.jeux.iter().any(|j| cle_chemin(&j.exe) == cle)
    }

    pub fn jeu_mut(&mut self, id: u64) -> Option<&mut Jeu> {
        self.jeux.iter_mut().find(|j| j.id == id)
    }
}

/// `true` pour un simple nom de fichier, sans dossier ni lecteur.
fn nom_de_fichier_simple(nom: &str) -> bool {
    !nom.is_empty() && nom != ".." && nom != "." && !nom.contains(['/', '\\', ':'])
}

/// Clé de comparaison de chemins insensible à la casse et aux séparateurs,
/// comme le système de fichiers Windows.
pub fn cle_chemin(p: &Path) -> String {
    p.to_string_lossy().replace('/', "\\").to_lowercase()
}

pub fn maintenant() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Emplacements des données de l'application.
#[derive(Debug, Clone)]
pub struct Dossiers {
    pub racine: PathBuf,
    pub config: PathBuf,
    pub icones: PathBuf,
    pub cache: PathBuf,
    pub sauvegardes: PathBuf,
    pub jaquettes: PathBuf,
    /// Clé SteamGridDB, hors de `config.json` pour ne se retrouver ni dans
    /// les sauvegardes ni dans les exports.
    pub cle_api: PathBuf,
}

impl Dossiers {
    /// Mode portable : `donnees/` à côté de l'exécutable. Si ce dossier n'est
    /// pas accessible en écriture (ex. Program Files), repli sur %APPDATA%.
    pub fn detecter() -> Self {
        let pres_exe = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|p| p.join("donnees")));

        let racine = pres_exe
            .filter(|d| dossier_inscriptible(d))
            .or_else(|| {
                std::env::var_os("APPDATA")
                    .map(|a| PathBuf::from(a).join("OmniLauncher"))
                    .filter(|d| dossier_inscriptible(d))
            })
            .unwrap_or_else(|| std::env::temp_dir().join("OmniLauncher"));

        Self::depuis_racine(racine)
    }

    pub fn depuis_racine(racine: PathBuf) -> Self {
        let d = Self {
            config: racine.join("config.json"),
            icones: racine.join("icones"),
            cache: racine.join("cache_icones"),
            sauvegardes: racine.join("sauvegardes"),
            jaquettes: racine.join("jaquettes"),
            cle_api: racine.join("cle_steamgriddb.txt"),
            racine,
        };
        let _ = fs::create_dir_all(&d.icones);
        let _ = fs::create_dir_all(&d.cache);
        let _ = fs::create_dir_all(&d.sauvegardes);
        let _ = fs::create_dir_all(&d.jaquettes);
        d
    }
}

fn dossier_inscriptible(d: &Path) -> bool {
    if fs::create_dir_all(d).is_err() {
        return false;
    }
    let test = d.join(".test_ecriture");
    let ok = fs::write(&test, b"ok").is_ok();
    let _ = fs::remove_file(&test);
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour_json() {
        let dir = std::env::temp_dir().join(format!("omni_test_cfg_{}", std::process::id()));
        let d = Dossiers::depuis_racine(dir.clone());
        let mut c = Config::default();
        let id = c.nouvel_id();
        c.jeux.push(Jeu::nouveau(
            id,
            "Test".into(),
            PathBuf::from("C:/Jeux/Test/test.exe"),
            None,
        ));
        c.sauvegarder(&d.config).unwrap();
        let relu = Config::charger(&d.config);
        assert_eq!(relu.jeux, c.jeux);
        assert_eq!(relu.prochain_id, 2);
        assert!(relu.contient_exe(Path::new("c:\\jeux\\test\\TEST.EXE")));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn config_corrompue_donne_defaut() {
        let dir = std::env::temp_dir().join(format!("omni_test_bad_{}", std::process::id()));
        let d = Dossiers::depuis_racine(dir.clone());
        fs::write(&d.config, "{ pas du json").unwrap();
        let c = Config::charger(&d.config);
        assert!(c.jeux.is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn config_v2_se_charge() {
        let v2 = r#"{"version":2,"taille_tuile":140.0,"fermer_apres_lancement":true,
            "tri":"Nom","dernier_dossier_scan":null,"prochain_id":2,
            "jeux":[{"id":1,"nom":"Ancien","exe":"C:/Jeux/a.exe","ajoute_le":5}]}"#;
        let c = Config::depuis_json(v2).unwrap();
        assert_eq!(c.version, VERSION_CONFIG);
        assert_eq!(c.theme, Theme::Omni);
        assert_eq!(c.jeux[0].temps_jeu, 0);
        assert!(c.jeux[0].tags.is_empty());
    }

    #[test]
    fn config_v3_se_charge() {
        let v3 = r#"{"version":3,"taille_tuile":140.0,"tri":"Nom","prochain_id":2,
            "jeux":[{"id":1,"nom":"Ancien","exe":"C:/Jeux/a.exe","tags":["Coop"]}]}"#;
        let c = Config::depuis_json(v3).unwrap();
        assert_eq!(c.version, VERSION_CONFIG);
        assert_eq!(c.affichage, Affichage::Grille);
        assert!(c.afficher_recents);
        assert!(c.verifier_maj);
        assert_eq!(c.jeux[0].statut, Statut::Aucun);
        assert!(c.jeux[0].notes.is_empty());
        assert!(c.jeux[0].jaquette.is_none());
    }

    #[test]
    fn un_nom_d_image_hors_du_dossier_est_oublie() {
        let json = r#"{"prochain_id":5,"jeux":[
            {"id":1,"nom":"A","exe":"C:/a.exe","jaquette":"C:\\Users\\moi\\doc.png"},
            {"id":2,"nom":"B","exe":"C:/b.exe","banniere":"..\\..\\config.json"},
            {"id":3,"nom":"C","exe":"C:/c.exe","icone_perso":"../secret.png"},
            {"id":4,"nom":"D","exe":"C:/d.exe","jaquette":"4_jaquette_1.png"}]}"#;
        let c = Config::depuis_json(json).unwrap();
        assert_eq!(c.jeux[0].jaquette, None);
        assert_eq!(c.jeux[1].banniere, None);
        assert_eq!(c.jeux[2].icone_perso, None);
        assert_eq!(c.jeux[3].jaquette.as_deref(), Some("4_jaquette_1.png"));
    }

    #[test]
    fn statut_et_notes_survivent_a_l_aller_retour() {
        let mut c = Config::default();
        c.jeux.push(Jeu::nouveau(1, "A".into(), PathBuf::from("C:/a.exe"), None));
        c.jeux[0].statut = Statut::Termine;
        c.jeux[0].notes = "Fin secrète à faire".into();
        let relu = Config::depuis_json(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(relu.jeux, c.jeux);
    }

    #[test]
    fn statuts_utilises_avec_leur_compte() {
        let mut c = Config::default();
        for (i, s) in [Statut::Termine, Statut::Aucun, Statut::Termine, Statut::AJouer]
            .into_iter()
            .enumerate()
        {
            let chemin = PathBuf::from(format!("C:/{i}.exe"));
            c.jeux.push(Jeu::nouveau(i as u64, "J".into(), chemin, None));
            c.jeux[i].statut = s;
        }
        assert_eq!(c.statuts(), vec![(Statut::AJouer, 1), (Statut::Termine, 2)]);
    }

    #[test]
    fn recents_tries_et_limites() {
        let mut c = Config::default();
        for (i, t) in [Some(10), None, Some(30), Some(20)].into_iter().enumerate() {
            let chemin = PathBuf::from(format!("C:/{i}.exe"));
            c.jeux.push(Jeu::nouveau(i as u64, "J".into(), chemin, None));
            c.jeux[i].dernier_lancement = t;
        }
        assert_eq!(c.recents(2), vec![2, 3]);
        assert_eq!(c.recents(10), vec![2, 3, 0], "jamais lancé : exclu");
    }

    #[test]
    fn tags_sans_doublon_et_renommage() {
        let mut c = Config::default();
        for nom in ["A", "B"] {
            let id = c.nouvel_id();
            c.jeux.push(Jeu::nouveau(id, nom.into(), PathBuf::from(format!("C:/{nom}.exe")), None));
        }
        assert!(c.jeux[0].ajouter_tag("Coop"));
        assert!(!c.jeux[0].ajouter_tag(" coop "));
        c.jeux[1].ajouter_tag("COOP");
        c.jeux[1].ajouter_tag("Rétro");
        assert_eq!(c.tags(), vec![("Coop".to_string(), 2), ("Rétro".to_string(), 1)]);
        c.renommer_tag("coop", "Multi");
        assert!(c.jeux.iter().all(|j| j.a_tag("multi")));
        c.supprimer_tag("Multi");
        assert_eq!(c.tags(), vec![("Rétro".to_string(), 1)]);
    }

    #[test]
    fn fusion_ignore_les_doublons() {
        let mut a = Config::default();
        let id = a.nouvel_id();
        a.jeux.push(Jeu::nouveau(id, "A".into(), PathBuf::from("C:/a.exe"), None));
        let mut b = Config::default();
        b.jeux.push(Jeu::nouveau(1, "A bis".into(), PathBuf::from("c:\\A.EXE"), None));
        b.jeux.push(Jeu::nouveau(2, "B".into(), PathBuf::from("C:/b.exe"), None));
        assert_eq!(a.fusionner(b), 1);
        assert_eq!(a.jeux.len(), 2);
        assert_ne!(a.jeux[0].id, a.jeux[1].id);
    }

    #[test]
    fn sessions_plafonnees_cumul_conserve() {
        let mut j = Jeu::nouveau(1, "A".into(), PathBuf::from("C:/a.exe"), None);
        for i in 0..(SESSIONS_MAX as u64 + 5) {
            j.ajouter_session(i, 10);
        }
        assert_eq!(j.sessions.len(), SESSIONS_MAX);
        assert_eq!(j.temps_jeu, 10 * (SESSIONS_MAX as u64 + 5));
        assert_eq!(j.sessions[0].debut, 5);
    }
}
