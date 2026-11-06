//! Navigateur de fichiers interne : la fenêtre de sélection de Windows n'est
//! pas utilisable à la manette.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EntreeFichier {
    pub(super) nom: String,
    pub(super) chemin: PathBuf,
    pub(super) dossier: bool,
}

/// Lecteurs présents sur la machine.
pub(super) fn lecteurs() -> Vec<EntreeFichier> {
    if cfg!(windows) {
        ('A'..='Z')
            .map(|l| PathBuf::from(format!("{l}:\\")))
            .filter(|p| p.is_dir())
            .map(|chemin| EntreeFichier {
                nom: chemin.display().to_string(),
                chemin,
                dossier: true,
            })
            .collect()
    } else {
        vec![EntreeFichier {
            nom: "/".into(),
            chemin: PathBuf::from("/"),
            dossier: true,
        }]
    }
}

/// Contenu d'un dossier : sous-dossiers d'abord, puis les fichiers dont
/// l'extension est acceptée. Sans extension, seuls les dossiers sont listés.
pub(super) fn lister(dossier: &Path, extensions: &[&str]) -> Vec<EntreeFichier> {
    let Ok(lecture) = std::fs::read_dir(dossier) else {
        return Vec::new();
    };
    let mut entrees: Vec<EntreeFichier> = lecture
        .flatten()
        .filter_map(|e| {
            let chemin = e.path();
            let est_dossier = chemin.is_dir();
            let accepte = est_dossier
                || chemin
                    .extension()
                    .map(|x| x.to_string_lossy().to_lowercase())
                    .is_some_and(|x| extensions.contains(&x.as_str()));
            accepte.then(|| EntreeFichier {
                nom: e.file_name().to_string_lossy().into_owned(),
                chemin,
                dossier: est_dossier,
            })
        })
        .filter(|e| !e.nom.starts_with('$'))
        .collect();
    entrees.sort_by_key(|e| (!e.dossier, e.nom.to_lowercase()));
    entrees
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn dossiers_d_abord_et_extensions_filtrees() {
        let d = std::env::temp_dir().join(format!("omni_fichiers_{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("Zeta")).unwrap();
        fs::create_dir_all(d.join("$Recycle")).unwrap();
        fs::write(d.join("jeu.EXE"), "x").unwrap();
        fs::write(d.join("alpha.exe"), "x").unwrap();
        fs::write(d.join("note.txt"), "x").unwrap();

        let noms: Vec<String> = lister(&d, &["exe"]).into_iter().map(|e| e.nom).collect();
        assert_eq!(noms, ["Zeta", "alpha.exe", "jeu.EXE"]);

        let noms: Vec<String> = lister(&d, &[]).into_iter().map(|e| e.nom).collect();
        assert_eq!(noms, ["Zeta"]);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn dossier_absent_liste_vide() {
        assert!(lister(Path::new("Z:/absent/du/disque"), &["exe"]).is_empty());
    }
}
