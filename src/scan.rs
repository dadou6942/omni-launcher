//! Scan d'un dossier de jeux : chaque sous-dossier est considéré comme un jeu.
//! Tous les `.exe` utiles sont proposés, le plus probable est pré-coché, et
//! les exécutables parasites (désinstalleurs, redistribuables, crash
//! reporters…) sont écartés.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use walkdir::WalkDir;

const PROFONDEUR_MAX: usize = 6;
const ENTREES_MAX_PAR_JEU: usize = 50_000;

#[derive(Debug, Clone)]
pub struct Candidat {
    pub exe: PathBuf,
    pub nom: String,
    /// Dossier du jeu (sous-dossier du dossier scanné) ou l'exe lui-même
    /// s'il est posé directement dans le dossier scanné.
    pub cible_suppression: PathBuf,
    /// Pré-sélectionné dans la fenêtre de revue.
    pub coche: bool,
    /// Libellé du groupe (nom du dossier du jeu).
    pub groupe: String,
    /// Déjà dans la bibliothèque : signalé, jamais importé.
    pub doublon: bool,
}

/// Mots qui, présents dans le nom d'un exe, indiquent un utilitaire.
const EXE_PARASITES: &[&str] = &[
    "unins",
    "uninst",
    "setup",
    "install",
    "redist",
    "vcredist",
    "vc_redist",
    "dxsetup",
    "dxwebsetup",
    "dotnet",
    "ndp4",
    "crash",
    "reporter",
    "report",
    "updater",
    "helper",
    "cefprocess",
    "cefsharp",
    "subprocess",
    "webhelper",
    "prereq",
    "beservice",
    "notification",
    "elevate",
    "physx",
    "oalinst",
    "directx",
    "cleanup",
    "touchup",
    "quicksfv",
    "7z",
    "dxdiag",
    "vulkaninfo",
    "unitycrashhandler",
    "zfgamebrowser",
    "sendlogs",
    "bugreport",
];

/// Dossiers ignorés entièrement pendant le parcours.
const DOSSIERS_PARASITES: &[&str] = &[
    "_commonredist",
    "commonredist",
    "redist",
    "redistributable",
    "redistributables",
    "directx",
    "dotnet",
    "vcredist",
    "__installer",
    "_installer",
    "installer",
    "installers",
    "prerequisites",
    "prereqs",
    "support",
    "__support",
    "thirdparty",
    "crashreportclient",
    "easyanticheat",
    "battleye",
    "uninstall",
    "crashpad",
    "monobleedingedge",
    "engine",
    "$recycle.bin",
    "system volume information",
];

/// Mots qui font baisser la priorité sans exclure l'exe.
const EXE_SECONDAIRES: &[&str] = &[
    "config",
    "settings",
    "editor",
    "server",
    "dedicated",
    "tool",
    "benchmark",
    "launcherpatcher",
    "modmanager",
    "debug",
];

pub fn est_exe_parasite(exe: &Path) -> bool {
    let stem = stem_minuscule(exe);
    EXE_PARASITES.iter().any(|m| stem.contains(m))
}

pub fn est_dossier_parasite(nom: &str) -> bool {
    let n = nom.to_lowercase();
    DOSSIERS_PARASITES.contains(&n.as_str())
}

fn est_exe(p: &Path) -> bool {
    p.extension()
        .map(|e| e.eq_ignore_ascii_case("exe"))
        .unwrap_or(false)
}

fn stem_minuscule(p: &Path) -> String {
    p.file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Ne garde que les lettres et chiffres, en minuscules.
fn normaliser(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Si l'utilisateur choisit une bibliothèque Steam, on descend
/// automatiquement dans `steamapps/common`.
pub fn resoudre_racine(racine: &Path) -> PathBuf {
    for sous in ["steamapps/common", "common"] {
        let candidat = racine.join(sous);
        let nom_ok = sous != "common"
            || racine
                .file_name()
                .map(|n| n.eq_ignore_ascii_case("steamapps"))
                .unwrap_or(false);
        if nom_ok && candidat.is_dir() {
            return candidat;
        }
    }
    racine.to_path_buf()
}

/// Note de « probabilité d'être l'exe principal » du jeu.
pub fn score(exe: &Path, dossier_jeu: &Path, taille: u64) -> i32 {
    let stem = stem_minuscule(exe);
    let stem_n = normaliser(&stem);
    let dossier_n = normaliser(
        &dossier_jeu
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );

    let profondeur = exe
        .strip_prefix(dossier_jeu)
        .map(|r| r.components().count().saturating_sub(1))
        .unwrap_or(0) as i32;

    let mut s = -15 * profondeur;

    if !stem_n.is_empty() && !dossier_n.is_empty() {
        if stem_n == dossier_n {
            s += 100;
        } else if dossier_n.contains(&stem_n) || stem_n.contains(&dossier_n) {
            s += 60;
        } else {
            let commun = stem_n
                .chars()
                .zip(dossier_n.chars())
                .take_while(|(a, b)| a == b)
                .count();
            if commun >= 4 {
                s += 20;
            }
        }
    }

    if EXE_SECONDAIRES.iter().any(|m| stem.contains(m)) {
        s -= 40;
    }
    if stem.contains("shipping") {
        s += 20;
    }

    let chemin = exe.to_string_lossy().to_lowercase();
    if chemin.contains("win64") || chemin.contains("x64") {
        s += 5;
    }
    if chemin.contains("win32") || chemin.contains("x86") {
        s -= 5;
    }

    s + ((taille / 1_000_000).min(200) / 10) as i32
}

/// Transforme un nom technique en nom lisible :
/// `Hollow_Knight-Win64-Shipping` → `Hollow Knight`.
pub fn embellir_nom(brut: &str) -> String {
    let mut s = brut.to_string();
    for suffixe in [
        "-win64-shipping",
        "-win32-shipping",
        "-shipping",
        "_x64",
        "_x86",
        "_64",
        "-x64",
        "_win64",
    ] {
        if s.to_lowercase().ends_with(suffixe) {
            s.truncate(s.len() - suffixe.len());
        }
    }

    // Séparateurs → espaces, et découpage CamelCase.
    let mut sortie = String::with_capacity(s.len() + 8);
    let mut precedent: Option<char> = None;
    for c in s.chars() {
        let c = if matches!(c, '_' | '-' | '.') { ' ' } else { c };
        if let Some(p) = precedent
            && c.is_uppercase()
            && p.is_lowercase()
        {
            sortie.push(' ');
        }
        sortie.push(c);
        precedent = Some(c);
    }
    let propre = sortie.split_whitespace().collect::<Vec<_>>().join(" ");
    if propre.is_empty() {
        brut.to_string()
    } else {
        propre
    }
}

/// Scanne `racine`. `deja_present` permet de signaler les exe déjà dans la
/// bibliothèque ; `progression(fait, total, nom)` est appelé pour chaque dossier.
pub fn scanner(
    racine: &Path,
    deja_present: impl Fn(&Path) -> bool,
    mut progression: impl FnMut(usize, usize, &str),
    annule: &AtomicBool,
) -> Vec<Candidat> {
    let racine = resoudre_racine(racine);
    let mut resultats = Vec::new();

    let Ok(lecture) = std::fs::read_dir(&racine) else {
        return resultats;
    };
    let mut entrees: Vec<PathBuf> = lecture.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entrees.sort_by_key(|p| p.to_string_lossy().to_lowercase());
    let total = entrees.len();

    for (i, chemin) in entrees.into_iter().enumerate() {
        if annule.load(Ordering::Relaxed) {
            break;
        }
        let nom_entree = chemin
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        progression(i, total, &nom_entree);

        // Exécutable posé directement dans le dossier scanné.
        if chemin.is_file() {
            if est_exe(&chemin) && !est_exe_parasite(&chemin) {
                let doublon = deja_present(&chemin);
                let stem = chemin
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                resultats.push(Candidat {
                    nom: embellir_nom(&stem),
                    cible_suppression: chemin.clone(),
                    exe: chemin,
                    coche: !doublon,
                    groupe: "(fichiers isolés)".into(),
                    doublon,
                });
            }
            continue;
        }

        if !chemin.is_dir() || est_dossier_parasite(&nom_entree) {
            continue;
        }

        let mut exes: Vec<(PathBuf, i32)> = WalkDir::new(&chemin)
            .max_depth(PROFONDEUR_MAX)
            .into_iter()
            .filter_entry(|e| {
                !(e.file_type().is_dir()
                    && e.depth() > 0
                    && est_dossier_parasite(&e.file_name().to_string_lossy()))
            })
            .take(ENTREES_MAX_PAR_JEU)
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file() && est_exe(e.path()) && !est_exe_parasite(e.path()))
            .map(|e| {
                let taille = e.metadata().map(|m| m.len()).unwrap_or(0);
                let s = score(e.path(), &chemin, taille);
                (e.into_path(), s)
            })
            .collect();

        if exes.is_empty() {
            continue;
        }
        // Meilleur score en premier, puis ordre alphabétique pour la stabilité.
        exes.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let nom_jeu = embellir_nom(&nom_entree);
        let plusieurs = exes.len() > 1;
        for (rang, (exe, _)) in exes.into_iter().enumerate() {
            let doublon = deja_present(&exe);
            let nom = if rang == 0 {
                nom_jeu.clone()
            } else {
                let stem = exe
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                format!("{nom_jeu} – {}", embellir_nom(&stem))
            };
            resultats.push(Candidat {
                exe,
                nom,
                cible_suppression: chemin.clone(),
                coche: !doublon && (rang == 0 || !plusieurs),
                groupe: nom_jeu.clone(),
                doublon,
            });
        }
    }
    progression(total, total, "");
    resultats
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn creer(base: &Path, rel: &str, octets: usize) {
        let p = base.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, vec![0u8; octets]).unwrap();
    }

    #[test]
    fn filtres() {
        assert!(est_exe_parasite(Path::new("unins000.exe")));
        assert!(est_exe_parasite(Path::new("UnityCrashHandler64.exe")));
        assert!(est_exe_parasite(Path::new("VC_redist.x64.exe")));
        assert!(!est_exe_parasite(Path::new("HollowKnight.exe")));
        assert!(est_dossier_parasite("_CommonRedist"));
        assert!(!est_dossier_parasite("Binaries"));
    }

    #[test]
    fn noms() {
        assert_eq!(embellir_nom("Hollow_Knight"), "Hollow Knight");
        assert_eq!(embellir_nom("HollowKnight"), "Hollow Knight");
        assert_eq!(embellir_nom("Stray-Win64-Shipping"), "Stray");
        assert_eq!(embellir_nom("Celeste"), "Celeste");
    }

    #[test]
    fn scan_complet() {
        let base = std::env::temp_dir().join(format!("omni_test_scan_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        // Jeu Unity
        creer(&base, "Hollow Knight/hollow_knight.exe", 1000);
        creer(&base, "Hollow Knight/UnityCrashHandler64.exe", 1000);
        creer(&base, "Hollow Knight/unins000.exe", 1000);
        // Jeu Unreal : lanceur racine + exe Shipping profond
        creer(&base, "Stray/Stray.exe", 500);
        creer(
            &base,
            "Stray/Hk_project/Binaries/Win64/Stray-Win64-Shipping.exe",
            5000,
        );
        creer(
            &base,
            "Stray/Engine/Binaries/Win64/CrashReportClient.exe",
            100,
        );
        // Redistribuables à ignorer
        creer(&base, "Celeste/Celeste.exe", 1000);
        creer(&base, "Celeste/_CommonRedist/DirectX/DXSETUP.exe", 100);
        // Exe isolé et dossier sans exe
        creer(&base, "Tetris.exe", 100);
        creer(&base, "Vide/lisezmoi.txt", 10);
        // Déjà présent dans la bibliothèque
        creer(&base, "Deja/Deja.exe", 100);

        let deja = base.join("Deja/Deja.exe");
        let annule = AtomicBool::new(false);
        let r = scanner(&base, |p| p == deja, |_, _, _| {}, &annule);

        let coches: Vec<_> = r
            .iter()
            .filter(|c| c.coche)
            .map(|c| c.nom.as_str())
            .collect();
        assert!(coches.contains(&"Hollow Knight"), "{coches:?}");
        assert!(coches.contains(&"Stray"));
        assert!(coches.contains(&"Celeste"));
        assert!(coches.contains(&"Tetris"));
        assert_eq!(coches.len(), 4, "{coches:?}");
        let doublons: Vec<_> = r.iter().filter(|c| c.doublon).collect();
        assert_eq!(doublons.len(), 1, "le doublon reste signalé");
        assert!(!doublons[0].coche, "un doublon n'est jamais pré-coché");

        // L'exe principal de Stray est celui à la racine.
        let stray = r.iter().find(|c| c.nom == "Stray").unwrap();
        assert!(stray.exe.ends_with("Stray/Stray.exe"));
        assert_eq!(stray.cible_suppression, base.join("Stray"));
        // L'exe Shipping est proposé mais non coché.
        assert!(
            r.iter()
                .any(|c| !c.coche && c.exe.ends_with("Stray-Win64-Shipping.exe"))
        );
        // Aucun parasite.
        assert!(r.iter().all(|c| !est_exe_parasite(&c.exe)));
        assert!(r.iter().all(|c| !c.exe.ends_with("Deja/Deja.exe")));
        // L'exe isolé ne doit jamais cibler le dossier scanné pour la suppression.
        let tetris = r.iter().find(|c| c.nom == "Tetris").unwrap();
        assert_eq!(tetris.cible_suppression, base.join("Tetris.exe"));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn bibliotheque_steam() {
        let base = std::env::temp_dir().join(format!("omni_test_steam_{}", std::process::id()));
        fs::create_dir_all(base.join("steamapps/common")).unwrap();
        assert_eq!(resoudre_racine(&base), base.join("steamapps/common"));
        assert_eq!(
            resoudre_racine(&base.join("steamapps")),
            base.join("steamapps/common")
        );
        let _ = fs::remove_dir_all(&base);
    }
}
