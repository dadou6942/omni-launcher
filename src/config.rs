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
}

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
        }
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub taille_tuile: f32,
    pub fermer_apres_lancement: bool,
    pub tri: Tri,
    pub dernier_dossier_scan: Option<PathBuf>,
    pub prochain_id: u64,
    pub jeux: Vec<Jeu>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 2,
            taille_tuile: 140.0,
            fermer_apres_lancement: true,
            tri: Tri::Nom,
            dernier_dossier_scan: None,
            prochain_id: 1,
            jeux: Vec::new(),
        }
    }
}

impl Config {
    pub fn charger(chemin: &Path) -> Self {
        let mut config: Config = match fs::read_to_string(chemin) {
            Ok(txt) => serde_json::from_str(&txt).unwrap_or_else(|_| {
                // Fichier illisible : on le met de côté au lieu de l'écraser.
                let _ = fs::copy(chemin, chemin.with_extension("json.bak"));
                Config::default()
            }),
            Err(_) => Config::default(),
        };
        config.taille_tuile = config
            .taille_tuile
            .clamp(TAILLE_TUILE_MIN, TAILLE_TUILE_MAX);
        // Sécurité si le fichier a été édité à la main.
        let max_id = config.jeux.iter().map(|j| j.id).max().unwrap_or(0);
        config.prochain_id = config.prochain_id.max(max_id + 1);
        config
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
            racine,
        };
        let _ = fs::create_dir_all(&d.icones);
        let _ = fs::create_dir_all(&d.cache);
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
}
