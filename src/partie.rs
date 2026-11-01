//! Suivi d'une partie pour mesurer le temps de jeu.
//!
//! On ne garde pas la main sur le processus lancé : beaucoup de jeux passent
//! par un petit lanceur qui démarre le vrai jeu puis se ferme. On surveille
//! donc tous les processus dont l'exécutable se trouve dans le dossier du jeu.

use crate::config::{cle_chemin, maintenant};
use eframe::egui;
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

/// Temps laissé au jeu pour apparaître (lanceurs, mises à jour, UAC…).
const DELAI_DETECTION: Duration = Duration::from_secs(90);
const INTERVALLE: Duration = Duration::from_secs(3);
/// Relevés consécutifs sans processus avant de considérer la partie finie :
/// tolère un lanceur qui passe la main au jeu.
const ABSENCES_AVANT_FIN: u32 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MsgPartie {
    /// Le jeu tourne (horodatage Unix de la détection).
    Debut(u64),
    Fin { debut: u64, fin: u64 },
    /// Aucun processus du jeu n'est apparu : temps non mesurable.
    NonDetectee,
}

/// Ce qui identifie les processus d'un jeu.
#[derive(Debug, Clone)]
pub struct Cible {
    exe: String,
    dossier: Option<String>,
    /// PID des processus qui tournaient déjà avant le lancement (un client
    /// comme Steam dans le même dossier, par exemple) : ils ne comptent pas.
    ignores: HashSet<u32>,
}

impl Cible {
    pub fn nouvelle(exe: &Path, dossier: Option<&Path>) -> Self {
        let windir = std::env::var_os("WINDIR").map(|w| dossier_cle(Path::new(&w)));
        Self {
            exe: cle_chemin(exe),
            // Un dossier trop proche de la racine, ou celui de Windows, attraperait
            // n'importe quel processus : on se contente alors de l'exe exact.
            dossier: dossier
                .filter(|d| {
                    d.components()
                        .filter(|c| matches!(c, Component::Normal(_)))
                        .count()
                        >= 2
                })
                .map(dossier_cle)
                .filter(|d| windir.as_ref().is_none_or(|w| !d.starts_with(w.as_str()))),
            ignores: HashSet::new(),
        }
    }

    /// À appeler juste avant le lancement, avec les processus actifs.
    pub fn ignorer_existants(&mut self, actifs: &[(u32, PathBuf)]) {
        self.ignores = actifs
            .iter()
            .filter(|(pid, p)| self.correspond(*pid, p))
            .map(|(pid, _)| *pid)
            .collect();
    }

    pub fn correspond(&self, pid: u32, chemin: &Path) -> bool {
        if self.ignores.contains(&pid) {
            return false;
        }
        let cle = cle_chemin(chemin);
        cle == self.exe || self.dossier.as_ref().is_some_and(|d| cle.starts_with(d))
    }
}

/// Clé d'un dossier terminée par un séparateur, pour tester l'inclusion.
fn dossier_cle(d: &Path) -> String {
    format!("{}\\", cle_chemin(d).trim_end_matches('\\'))
}

/// Lance la surveillance dans un thread. Les messages arrivent dans l'ordre
/// `Debut` puis `Fin`, ou bien `NonDetectee` seul.
pub fn surveiller(cible: Cible, ctx: egui::Context) -> Receiver<MsgPartie> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let mut suivi = Suivi::nouveau(Instant::now());
        loop {
            let actif = crate::platform::processus_actifs()
                .iter()
                .any(|(pid, p)| cible.correspond(*pid, p));
            let msg = suivi.releve(actif, maintenant(), Instant::now());
            let fini = matches!(msg, Some(MsgPartie::Fin { .. } | MsgPartie::NonDetectee));
            if let Some(m) = msg {
                if tx.send(m).is_err() {
                    return; // Le launcher a été fermé.
                }
                ctx.request_repaint();
            }
            if fini {
                return;
            }
            std::thread::sleep(INTERVALLE);
        }
    });
    rx
}

/// Machine à états de la surveillance, séparée du thread pour être testable.
struct Suivi {
    lancement: Instant,
    debut: Option<u64>,
    dernier_vu: u64,
    absences: u32,
}

impl Suivi {
    fn nouveau(lancement: Instant) -> Self {
        Self {
            lancement,
            debut: None,
            dernier_vu: 0,
            absences: 0,
        }
    }

    fn releve(&mut self, actif: bool, horloge: u64, instant: Instant) -> Option<MsgPartie> {
        match (actif, self.debut) {
            (true, None) => {
                self.debut = Some(horloge);
                self.dernier_vu = horloge;
                Some(MsgPartie::Debut(horloge))
            }
            (true, Some(_)) => {
                self.dernier_vu = horloge;
                self.absences = 0;
                None
            }
            (false, None) => (instant.duration_since(self.lancement) > DELAI_DETECTION)
                .then_some(MsgPartie::NonDetectee),
            (false, Some(debut)) => {
                self.absences += 1;
                (self.absences >= ABSENCES_AVANT_FIN).then_some(MsgPartie::Fin {
                    debut,
                    fin: self.dernier_vu.max(debut),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correspondance_par_dossier_et_par_exe() {
        let c = Cible::nouvelle(
            Path::new("D:/Jeux/Hades/Launcher.exe"),
            Some(Path::new("D:/Jeux/Hades")),
        );
        assert!(c.correspond(1, Path::new("d:\\jeux\\hades\\x64\\Hades.exe")));
        assert!(c.correspond(1, Path::new("D:\\Jeux\\Hades\\Launcher.exe")));
        assert!(!c.correspond(1, Path::new("D:\\Jeux\\Hades II\\Hades2.exe")));
        assert!(!c.correspond(1, Path::new("C:\\Windows\\explorer.exe")));
    }

    #[test]
    fn processus_deja_lances_ignores_par_pid() {
        let mut c = Cible::nouvelle(
            Path::new("C:/Steam/outils/jeu.exe"),
            Some(Path::new("C:/Steam/outils")),
        );
        c.ignorer_existants(&[
            (10, PathBuf::from("C:/Steam/outils/client.exe")),
            (11, PathBuf::from("C:/Steam/outils/jeu.exe")),
            (12, PathBuf::from("C:/Windows/explorer.exe")),
        ]);
        assert!(!c.correspond(10, Path::new("c:\\steam\\outils\\CLIENT.exe")));
        assert!(!c.correspond(11, Path::new("C:/Steam/outils/jeu.exe")));
        // Une nouvelle instance du même exe compte, elle.
        assert!(c.correspond(20, Path::new("C:/Steam/outils/jeu.exe")));
    }

    #[test]
    fn dossier_trop_court_ignore() {
        let c = Cible::nouvelle(Path::new("D:/Jeux/jeu.exe"), Some(Path::new("D:/Jeux")));
        assert!(c.correspond(1, Path::new("D:/Jeux/jeu.exe")));
        assert!(!c.correspond(1, Path::new("D:/Jeux/autre.exe")));
    }

    #[cfg(windows)]
    #[test]
    fn dossier_de_windows_ignore() {
        let windir = PathBuf::from(std::env::var_os("WINDIR").unwrap());
        let sys = windir.join("System32");
        let c = Cible::nouvelle(&sys.join("cmd.exe"), Some(&sys));
        assert!(c.correspond(1, &sys.join("cmd.exe")));
        assert!(!c.correspond(1, &sys.join("svchost.exe")));
    }

    #[test]
    fn partie_complete_avec_passage_de_relais() {
        let t0 = Instant::now();
        let mut s = Suivi::nouveau(t0);
        assert_eq!(s.releve(false, 100, t0), None);
        assert_eq!(s.releve(true, 103, t0), Some(MsgPartie::Debut(103)));
        assert_eq!(s.releve(true, 200, t0), None);
        // Le lanceur se ferme, le jeu reprend juste après : pas de fin.
        assert_eq!(s.releve(false, 203, t0), None);
        assert_eq!(s.releve(true, 206, t0), None);
        for h in [300, 303, 306] {
            assert_eq!(s.releve(false, h, t0), None);
        }
        assert_eq!(
            s.releve(false, 309, t0),
            Some(MsgPartie::Fin { debut: 103, fin: 206 })
        );
    }

    #[test]
    fn jeu_jamais_vu() {
        let t0 = Instant::now();
        let mut s = Suivi::nouveau(t0);
        assert_eq!(s.releve(false, 0, t0 + Duration::from_secs(10)), None);
        assert_eq!(
            s.releve(false, 0, t0 + DELAI_DETECTION + Duration::from_secs(1)),
            Some(MsgPartie::NonDetectee)
        );
    }
}
