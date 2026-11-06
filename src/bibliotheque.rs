//! Couche métier : la bibliothèque de jeux et ses opérations. Elle ne dépend
//! pas de l'interface : le mode bureau et le mode plein écran appellent les
//! mêmes fonctions.

use crate::config::{Config, Dossiers, Jeu, Statut, Tri, cle_chemin, maintenant};
use crate::icons::Genre;
use crate::partie::Cible;
use crate::platform;
use crate::scan::{self, Candidat};
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

/// Partie de la bibliothèque affichée.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Section {
    #[default]
    Tout,
    Recents,
    Favoris,
    /// Une collection est un tag.
    Collection(String),
}

/// Critères d'affichage d'une liste de jeux.
#[derive(Debug, Clone, Copy)]
pub struct Filtre<'a> {
    pub section: &'a Section,
    pub recherche: &'a str,
    pub statut: Option<Statut>,
}

/// Pourquoi un jeu n'a pas pu démarrer.
#[derive(Debug, PartialEq, Eq)]
pub enum ErreurLancement {
    JeuInconnu,
    Introuvable,
    Refus { nom: String, raison: String },
}

/// Jeu lancé : de quoi suivre la partie.
pub struct Lancement {
    pub nom: String,
    pub cible: Cible,
}

/// Bilan d'un ajout de jeux.
#[derive(Debug, PartialEq, Eq)]
pub struct Ajout {
    pub nombre: usize,
    pub dernier_nom: String,
}

pub struct Bibliotheque {
    pub dossiers: Dossiers,
    pub config: Config,
    /// Jeux dont l'exécutable n'existe plus.
    pub manquants: HashSet<u64>,
}

impl Bibliotheque {
    pub fn nouvelle(dossiers: Dossiers, config: Config) -> Self {
        let mut bib = Self {
            dossiers,
            config,
            manquants: HashSet::new(),
        };
        bib.verifier_manquants();
        bib
    }

    pub fn sauver(&self) -> std::io::Result<()> {
        self.config.sauvegarder(&self.dossiers.config)
    }

    pub fn jeu(&self, id: u64) -> Option<&Jeu> {
        self.config.jeux.iter().find(|j| j.id == id)
    }

    pub fn verifier_manquants(&mut self) {
        self.manquants = self
            .config
            .jeux
            .iter()
            .filter(|j| !j.exe.is_file())
            .map(|j| j.id)
            .collect();
    }

    // ------------------------------------------------------------------
    // Listes
    // ------------------------------------------------------------------

    /// Jeux filtrés (recherche sur le nom et les tags) et triés. La section
    /// Récents suit toujours l'ordre des derniers lancements.
    pub fn lister(&self, filtre: Filtre<'_>) -> Vec<u64> {
        self.lister_jeux(filtre).into_iter().map(|j| j.id).collect()
    }

    /// Comme [`Self::lister`], sans repasser par les identifiants.
    pub fn lister_jeux(&self, filtre: Filtre<'_>) -> Vec<&Jeu> {
        let termes: Vec<String> = filtre
            .recherche
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();

        let mut jeux: Vec<&Jeu> = self
            .config
            .jeux
            .iter()
            .filter(|j| match filtre.section {
                Section::Tout => true,
                Section::Recents => j.dernier_lancement.is_some(),
                Section::Favoris => j.favori,
                Section::Collection(t) => j.a_tag(t),
            })
            .filter(|j| filtre.statut.is_none_or(|s| j.statut == s))
            .filter(|j| {
                let nom = j.nom.to_lowercase();
                termes.iter().all(|t| {
                    nom.contains(t.as_str())
                        || j.tags.iter().any(|g| g.to_lowercase().contains(t.as_str()))
                })
            })
            .collect();

        let tri = match filtre.section {
            Section::Recents => Tri::DernierLancement,
            _ => self.config.tri,
        };
        match tri {
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
        jeux
    }

    /// Sépare une liste en (favoris, autres), en gardant l'ordre.
    pub fn separer_favoris(&self, ids: &[u64]) -> (Vec<u64>, Vec<u64>) {
        ids.iter()
            .copied()
            .partition(|&id| self.jeu(id).is_some_and(|j| j.favori))
    }

    // ------------------------------------------------------------------
    // Ajouts
    // ------------------------------------------------------------------

    pub fn ajouter_exes(&mut self, exes: Vec<PathBuf>) -> Ajout {
        let mut ajout = Ajout {
            nombre: 0,
            dernier_nom: String::new(),
        };
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
            ajout.dernier_nom = nom.clone();
            self.config.jeux.push(Jeu::nouveau(id, nom, exe, None));
            ajout.nombre += 1;
        }
        self.verifier_manquants();
        ajout
    }

    /// Importe les candidats cochés d'un scan. Renvoie le nombre de jeux ajoutés.
    pub fn importer_candidats(&mut self, candidats: Vec<Candidat>) -> usize {
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
        self.verifier_manquants();
        n
    }

    /// Exécutables de la bibliothèque, pour reconnaître les doublons d'un scan.
    pub fn cles_exes(&self) -> HashSet<String> {
        self.config.jeux.iter().map(|j| cle_chemin(&j.exe)).collect()
    }

    // ------------------------------------------------------------------
    // Modifications d'un jeu
    // ------------------------------------------------------------------

    pub fn basculer_favori(&mut self, id: u64) {
        if let Some(j) = self.config.jeu_mut(id) {
            j.favori = !j.favori;
        }
    }

    /// Renomme un jeu. Un nom vide est ignoré.
    pub fn renommer(&mut self, id: u64, nom: &str) -> bool {
        let nom = nom.trim();
        if nom.is_empty() {
            return false;
        }
        match self.config.jeu_mut(id) {
            Some(j) => {
                j.nom = nom.to_string();
                true
            }
            None => false,
        }
    }

    pub fn definir_arguments(&mut self, id: u64, arguments: &str) {
        if let Some(j) = self.config.jeu_mut(id) {
            j.arguments = arguments.trim().to_string();
        }
    }

    pub fn definir_statut(&mut self, id: u64, statut: Statut) {
        if let Some(j) = self.config.jeu_mut(id) {
            j.statut = statut;
        }
    }

    pub fn definir_notes(&mut self, id: u64, notes: &str) {
        if let Some(j) = self.config.jeu_mut(id) {
            j.notes = notes.trim().to_string();
        }
    }

    pub fn relocaliser(&mut self, id: u64, exe: PathBuf) {
        if let Some(j) = self.config.jeu_mut(id) {
            j.exe = exe;
            j.cible_suppression = None;
        }
        self.verifier_manquants();
    }

    /// Adopte une nouvelle icône et supprime l'ancienne.
    pub fn changer_icone(&mut self, id: u64, nom: Option<String>) {
        let ancien = match self.config.jeu_mut(id) {
            Some(j) => std::mem::replace(&mut j.icone_perso, nom),
            None => None,
        };
        if let Some(a) = ancien {
            let _ = std::fs::remove_file(self.dossiers.icones.join(a));
        }
    }

    /// Remplace (ou retire, avec `None`) une image d'un jeu et supprime
    /// l'ancien fichier. Le jeu a pu disparaître pendant un téléchargement.
    pub fn changer_visuel(&mut self, id: u64, genre: Genre, nom: Option<String>) {
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
    }

    /// Retire un jeu de la bibliothèque et supprime ses images.
    pub fn retirer(&mut self, id: u64) -> Option<Jeu> {
        let pos = self.config.jeux.iter().position(|j| j.id == id)?;
        let jeu = self.config.jeux.remove(pos);
        if let Some(nom) = &jeu.icone_perso {
            let _ = std::fs::remove_file(self.dossiers.icones.join(nom));
        }
        for nom in [&jeu.jaquette, &jeu.banniere].into_iter().flatten() {
            let _ = std::fs::remove_file(self.dossiers.jaquettes.join(nom));
        }
        self.manquants.remove(&id);
        Some(jeu)
    }

    /// Ids des jeux dont l'exécutable a disparu.
    pub fn ids_introuvables(&self) -> Vec<u64> {
        self.config
            .jeux
            .iter()
            .filter(|j| !j.exe.is_file())
            .map(|j| j.id)
            .collect()
    }

    /// Oublie les images d'une configuration absentes de ce PC, avant de
    /// l'adopter.
    pub fn oublier_images_absentes(&self, config: &mut Config) {
        for j in &mut config.jeux {
            if j.icone_perso
                .as_ref()
                .is_some_and(|n| !self.dossiers.icones.join(n).is_file())
            {
                j.icone_perso = None;
            }
            for image in [&mut j.jaquette, &mut j.banniere] {
                if image
                    .as_ref()
                    .is_some_and(|n| !self.dossiers.jaquettes.join(n).is_file())
                {
                    *image = None;
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Lancement
    // ------------------------------------------------------------------

    /// Lance un jeu. `suivi_existant` indique qu'une partie de ce jeu est déjà
    /// suivie : les processus actifs comptent alors comme les siens.
    pub fn lancer(&mut self, id: u64, suivi_existant: bool) -> Result<Lancement, ErreurLancement> {
        let jeu = self.jeu(id).cloned().ok_or(ErreurLancement::JeuInconnu)?;
        if !jeu.exe.is_file() {
            self.manquants.insert(id);
            return Err(ErreurLancement::Introuvable);
        }
        let mut cible = Cible::nouvelle(&jeu.exe, jeu.dossier_jeu().as_deref());
        if !suivi_existant {
            cible.ignorer_existants(&platform::processus_actifs());
        }
        platform::lancer(&jeu.exe, &jeu.arguments).map_err(|raison| ErreurLancement::Refus {
            nom: jeu.nom.clone(),
            raison,
        })?;
        if let Some(j) = self.config.jeu_mut(id) {
            j.dernier_lancement = Some(maintenant());
            j.nb_lancements += 1;
        }
        Ok(Lancement {
            nom: jeu.nom,
            cible,
        })
    }

    // ------------------------------------------------------------------
    // Suppression du disque
    // ------------------------------------------------------------------

    /// Garde-fous avant d'envoyer un dossier à la corbeille.
    pub fn verifier_suppression(&self, jeu: &Jeu) -> Result<PathBuf, String> {
        let cible = jeu
            .dossier_a_supprimer()
            .ok_or("Impossible de déterminer le dossier du jeu.")?;

        if !cible.exists() {
            return Err("Le dossier du jeu n'existe plus sur le disque.".into());
        }
        if !cible.is_absolute() {
            return Err("Chemin relatif refusé par sécurité.".into());
        }
        // Un « .. » pourrait contourner la comparaison avec les dossiers protégés.
        if cible.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err("Chemin avec « .. » refusé par sécurité.".into());
        }
        let profondeur = cible
            .components()
            .filter(|c| matches!(c, Component::Normal(_)))
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn bib_test(nom: &str, jeux: &[(&str, Statut, bool, Option<u64>)]) -> Bibliotheque {
        let dir = std::env::temp_dir().join(format!("omni_bib_{nom}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let mut config = Config::default();
        for (i, (n, statut, favori, lance)) in jeux.iter().enumerate() {
            let id = config.nouvel_id();
            let exe = PathBuf::from(format!("C:/absent/{i}.exe"));
            let mut j = Jeu::nouveau(id, (*n).into(), exe, None);
            j.statut = *statut;
            j.favori = *favori;
            j.dernier_lancement = *lance;
            config.jeux.push(j);
        }
        Bibliotheque::nouvelle(Dossiers::depuis_racine(dir), config)
    }

    fn filtre<'a>(section: &'a Section, recherche: &'a str) -> Filtre<'a> {
        Filtre {
            section,
            recherche,
            statut: None,
        }
    }

    #[test]
    fn liste_triee_par_nom_et_recherche_sur_les_tags() {
        let mut b = bib_test(
            "tri",
            &[("Zelda", Statut::Aucun, false, None), ("Astro", Statut::Aucun, false, None)],
        );
        b.config.jeux[0].ajouter_tag("Coop");
        assert_eq!(b.lister(filtre(&Section::Tout, "")), vec![2, 1]);
        assert_eq!(b.lister(filtre(&Section::Tout, "coop")), vec![1]);
        assert_eq!(b.lister(filtre(&Section::Tout, "astro zelda")), Vec::<u64>::new());
    }

    #[test]
    fn sections_favoris_collection_et_recents() {
        let mut b = bib_test(
            "sections",
            &[
                ("A", Statut::Aucun, true, Some(10)),
                ("B", Statut::Aucun, false, Some(30)),
                ("C", Statut::Aucun, false, None),
            ],
        );
        b.config.jeux[2].ajouter_tag("Rétro");
        assert_eq!(b.lister(filtre(&Section::Favoris, "")), vec![1]);
        assert_eq!(b.lister(filtre(&Section::Collection("Rétro".into()), "")), vec![3]);
        assert_eq!(b.lister(filtre(&Section::Recents, "")), vec![2, 1]);
    }

    #[test]
    fn filtre_de_statut() {
        let b = bib_test(
            "statut",
            &[("A", Statut::Termine, false, None), ("B", Statut::EnCours, false, None)],
        );
        let f = Filtre {
            section: &Section::Tout,
            recherche: "",
            statut: Some(Statut::EnCours),
        };
        assert_eq!(b.lister(f), vec![2]);
    }

    #[test]
    fn favoris_separes_sans_changer_l_ordre() {
        let b = bib_test(
            "fav",
            &[
                ("A", Statut::Aucun, false, None),
                ("B", Statut::Aucun, true, None),
                ("C", Statut::Aucun, false, None),
            ],
        );
        assert_eq!(b.separer_favoris(&[3, 2, 1]), (vec![2], vec![3, 1]));
    }

    #[test]
    fn ajout_ignore_les_doublons() {
        let mut b = bib_test("ajout", &[("A", Statut::Aucun, false, None)]);
        let bilan = b.ajouter_exes(vec![
            PathBuf::from("C:/absent/0.exe"),
            PathBuf::from("C:/absent/nouveau.exe"),
        ]);
        assert_eq!(bilan.nombre, 1);
        assert_eq!(b.config.jeux.len(), 2);
        assert_eq!(b.manquants.len(), 2);
    }

    #[test]
    fn renommer_refuse_un_nom_vide() {
        let mut b = bib_test("nom", &[("A", Statut::Aucun, false, None)]);
        assert!(!b.renommer(1, "   "));
        assert!(b.renommer(1, " Nouveau "));
        assert_eq!(b.jeu(1).unwrap().nom, "Nouveau");
    }

    #[test]
    fn lancer_un_jeu_absent_le_marque_introuvable() {
        let mut b = bib_test("lancer", &[("A", Statut::Aucun, false, None)]);
        b.manquants.clear();
        assert_eq!(b.lancer(1, false).err(), Some(ErreurLancement::Introuvable));
        assert!(b.manquants.contains(&1));
        assert_eq!(b.lancer(99, false).err(), Some(ErreurLancement::JeuInconnu));
    }

    #[test]
    fn retirer_supprime_les_images() {
        let mut b = bib_test("retirer", &[("A", Statut::Aucun, false, None)]);
        let image = b.dossiers.jaquettes.join("1_jaquette_1.png");
        fs::write(&image, "x").unwrap();
        b.changer_visuel(1, Genre::Jaquette, Some("1_jaquette_1.png".into()));
        assert!(b.retirer(1).is_some());
        assert!(!image.exists());
        assert!(b.retirer(1).is_none());
    }

    #[test]
    fn changer_visuel_supprime_l_ancien_fichier_et_garde_le_meme_nom() {
        let mut b = bib_test("visuel", &[("A", Statut::Aucun, false, None)]);
        let (ancien, nouveau) = (
            b.dossiers.jaquettes.join("a.png"),
            b.dossiers.jaquettes.join("b.png"),
        );
        fs::write(&ancien, "x").unwrap();
        fs::write(&nouveau, "x").unwrap();
        b.changer_visuel(1, Genre::Banniere, Some("a.png".into()));
        b.changer_visuel(1, Genre::Banniere, Some("a.png".into()));
        assert!(ancien.exists(), "même nom : le fichier reste");
        b.changer_visuel(1, Genre::Banniere, Some("b.png".into()));
        assert!(!ancien.exists());
        b.changer_visuel(42, Genre::Banniere, Some("b.png".into()));
        assert!(!nouveau.exists(), "jeu disparu : l'image est nettoyée");
    }

    #[test]
    fn suppression_refusee_pour_un_chemin_avec_remontee() {
        let mut b = bib_test("remontee", &[("A", Statut::Aucun, false, None)]);
        let dossier = b.dossiers.racine.join("jeu");
        fs::create_dir_all(&dossier).unwrap();
        let piege = b.dossiers.racine.join("jeu").join("..").join("jeu");
        b.config.jeux[0].exe = piege.join("a.exe");
        b.config.jeux[0].cible_suppression = Some(piege);
        let jeu = b.jeu(1).unwrap().clone();
        let erreur = b.verifier_suppression(&jeu).unwrap_err();
        assert!(erreur.contains("«"), "{erreur}");
    }

    #[test]
    fn suppression_refusee_pour_un_dossier_protege() {
        let mut b = bib_test("garde", &[("A", Statut::Aucun, false, None)]);
        b.config.jeux[0].cible_suppression = Some(PathBuf::from("Z:/absent"));
        let jeu = b.jeu(1).unwrap().clone();
        assert!(b.verifier_suppression(&jeu).is_err());

        // Le dossier du launcher lui-même est protégé.
        b.config.jeux[0].cible_suppression = Some(b.dossiers.racine.clone());
        b.config.jeux[0].exe = b.dossiers.racine.join("jeu.exe");
        let jeu = b.jeu(1).unwrap().clone();
        let erreur = b.verifier_suppression(&jeu).unwrap_err();
        assert!(erreur.contains("protégé"), "{erreur}");
    }
}
