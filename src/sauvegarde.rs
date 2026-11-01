//! Sauvegardes de la configuration : une copie automatique par jour (les
//! dernières sont conservées), copies de sécurité avant une restauration ou
//! un import, et export/import manuels.

use crate::config::{Config, Dossiers};
use chrono::{Local, TimeZone};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub const SAUVEGARDES_MAX: usize = 10;

#[derive(Debug, Clone)]
pub struct InfoSauvegarde {
    pub chemin: PathBuf,
    /// Date de création, horodatage Unix.
    pub date: u64,
    /// Nombre de jeux, ou `None` si le fichier est illisible.
    pub nb_jeux: Option<usize>,
}

/// Copie la configuration du jour si elle n'existe pas encore.
/// Renvoie `true` si une copie a été créée.
pub fn sauvegarde_du_jour(d: &Dossiers) -> std::io::Result<bool> {
    if !d.config.is_file() {
        return Ok(false);
    }
    let cible = d
        .sauvegardes
        .join(format!("config_{}.json", Local::now().format("%Y-%m-%d")));
    if cible.exists() {
        return Ok(false);
    }
    fs::copy(&d.config, &cible)?;
    elaguer(&d.sauvegardes, SAUVEGARDES_MAX);
    Ok(true)
}

/// Copie de sécurité immédiate, par exemple avant une restauration.
pub fn sauvegarde_immediate(d: &Dossiers, etiquette: &str) -> std::io::Result<()> {
    if !d.config.is_file() {
        return Ok(());
    }
    let nom = format!(
        "config_{}_{etiquette}.json",
        Local::now().format("%Y-%m-%d_%H%M%S")
    );
    fs::copy(&d.config, d.sauvegardes.join(nom))?;
    elaguer(&d.sauvegardes, SAUVEGARDES_MAX);
    Ok(())
}

/// Sauvegardes existantes, les plus récentes d'abord.
pub fn lister(dossier: &Path) -> Vec<InfoSauvegarde> {
    let mut liste: Vec<InfoSauvegarde> = fichiers(dossier)
        .into_iter()
        .map(|(chemin, date)| {
            let nb_jeux = fs::read_to_string(&chemin)
                .ok()
                .and_then(|t| Config::depuis_json(&t).ok())
                .map(|c| c.jeux.len());
            InfoSauvegarde {
                chemin,
                date,
                nb_jeux,
            }
        })
        .collect();
    liste.sort_by_key(|s| std::cmp::Reverse(s.date));
    liste
}

pub fn lire(chemin: &Path) -> Result<Config, String> {
    let texte = fs::read_to_string(chemin).map_err(|e| format!("Lecture impossible : {e}"))?;
    Config::depuis_json(&texte).map_err(|e| format!("Ce fichier n'est pas une configuration valide : {e}"))
}

pub fn exporter(config: &Config, chemin: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(chemin, json).map_err(|e| e.to_string())
}

/// Texte lisible pour une date de sauvegarde, en heure locale.
pub fn libelle_date(ts: u64) -> String {
    match Local.timestamp_opt(ts as i64, 0).single() {
        Some(d) => d.format("%d/%m/%Y à %H:%M").to_string(),
        None => "date inconnue".into(),
    }
}

fn fichiers(dossier: &Path) -> Vec<(PathBuf, u64)> {
    let Ok(entrees) = fs::read_dir(dossier) else {
        return Vec::new();
    };
    entrees
        .flatten()
        .filter(|e| {
            let nom = e.file_name().to_string_lossy().to_lowercase();
            nom.starts_with("config_") && nom.ends_with(".json")
        })
        .map(|e| {
            let date = e
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            (e.path(), date)
        })
        .collect()
}

/// Ne garde que les `max` sauvegardes les plus récentes.
fn elaguer(dossier: &Path, max: usize) {
    let mut liste = fichiers(dossier);
    liste.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.cmp(&a.0)));
    for (chemin, _) in liste.into_iter().skip(max) {
        let _ = fs::remove_file(chemin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Jeu;

    #[test]
    fn une_copie_par_jour_et_elagage() {
        let dir = std::env::temp_dir().join(format!("omni_test_sauv_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let d = Dossiers::depuis_racine(dir.clone());
        assert!(!sauvegarde_du_jour(&d).unwrap(), "pas de config, pas de copie");

        let mut c = Config::default();
        c.jeux.push(Jeu::nouveau(1, "A".into(), PathBuf::from("C:/a.exe"), None));
        c.sauvegarder(&d.config).unwrap();
        assert!(sauvegarde_du_jour(&d).unwrap());
        assert!(!sauvegarde_du_jour(&d).unwrap(), "une seule copie par jour");

        let liste = lister(&d.sauvegardes);
        assert_eq!(liste.len(), 1);
        assert_eq!(liste[0].nb_jeux, Some(1));

        for i in 0..(SAUVEGARDES_MAX + 3) {
            fs::write(d.sauvegardes.join(format!("config_vieux_{i:02}.json")), "{}").unwrap();
        }
        elaguer(&d.sauvegardes, SAUVEGARDES_MAX);
        assert_eq!(fichiers(&d.sauvegardes).len(), SAUVEGARDES_MAX);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn export_puis_lecture() {
        let dir = std::env::temp_dir().join(format!("omni_test_export_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut c = Config::default();
        c.jeux.push(Jeu::nouveau(1, "A".into(), PathBuf::from("C:/a.exe"), None));
        c.jeux[0].ajouter_tag("Coop");
        let f = dir.join("export.json");
        exporter(&c, &f).unwrap();
        let relu = lire(&f).unwrap();
        assert_eq!(relu.jeux, c.jeux);
        assert!(lire(&dir.join("absent.json")).is_err());
        let _ = fs::remove_dir_all(dir);
    }
}
