//! Jaquettes et bannières : recherche et téléchargement sur SteamGridDB.
//!
//! La clé d'API est celle de l'utilisateur. Elle n'est envoyée qu'à l'API, pas
//! au CDN qui sert les images, et n'apparaît jamais dans les messages d'erreur.

use crate::config::{Dossiers, maintenant};
use crate::icons::Genre;
use crate::reseau;
use serde::Deserialize;
use std::fs;
use std::io::Cursor;
use std::path::Path;

const API: &str = "https://www.steamgriddb.com/api/v2";
pub const PAGE_CLE_API: &str = "https://www.steamgriddb.com/profile/preferences/api";
const TAILLE_IMAGE_MAX: u64 = 30 * 1024 * 1024;
/// Combien de jeux proposer après une recherche.
const RESULTATS_MAX: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultatJeu {
    pub id: u64,
    pub nom: String,
}

/// Noms des fichiers créés dans le dossier `jaquettes/`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Telecharges {
    pub jaquette: Option<String>,
    pub banniere: Option<String>,
}

#[derive(Deserialize)]
struct Enveloppe<T> {
    #[serde(default = "Vec::new")]
    data: Vec<T>,
}

#[derive(Deserialize)]
struct JeuBrut {
    id: u64,
    name: String,
}

#[derive(Deserialize)]
struct ImageBrute {
    url: String,
}

pub fn lire_cle(d: &Dossiers) -> String {
    fs::read_to_string(&d.cle_api)
        .map(|c| c.trim().to_string())
        .unwrap_or_default()
}

/// Enregistre la clé, ou retire le fichier si elle est vide.
pub fn ecrire_cle(d: &Dossiers, cle: &str) -> std::io::Result<()> {
    let cle = cle.trim();
    if cle.is_empty() {
        return match fs::remove_file(&d.cle_api) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }
    fs::write(&d.cle_api, cle)
}

/// Encode un texte pour l'insérer comme segment de chemin d'une URL.
fn encoder_segment(texte: &str) -> String {
    let mut sortie = String::with_capacity(texte.len());
    for octet in texte.bytes() {
        if octet.is_ascii_alphanumeric() || matches!(octet, b'-' | b'_' | b'.' | b'~') {
            sortie.push(octet as char);
        } else {
            sortie.push_str(&format!("%{octet:02X}"));
        }
    }
    sortie
}

fn appel_api(url: &str, cle: &str) -> Result<Option<String>, String> {
    let (statut, texte) = reseau::lire_texte(url, Some(cle))?;
    match statut {
        200 => Ok(Some(texte)),
        404 => Ok(None),
        401 | 403 => Err("Clé SteamGridDB refusée. Vérifiez-la dans les paramètres.".into()),
        429 => Err("SteamGridDB limite les requêtes, réessayez plus tard.".into()),
        code => Err(format!("SteamGridDB a répondu avec le code {code}.")),
    }
}

fn analyser_recherche(json: &str) -> Result<Vec<ResultatJeu>, String> {
    let reponse: Enveloppe<JeuBrut> =
        serde_json::from_str(json).map_err(|e| format!("Réponse de SteamGridDB illisible : {e}"))?;
    Ok(reponse
        .data
        .into_iter()
        .take(RESULTATS_MAX)
        .map(|j| ResultatJeu { id: j.id, nom: j.name })
        .collect())
}

/// Première image servie par SteamGridDB (l'API les trie par score).
fn premiere_image(json: &str) -> Result<Option<String>, String> {
    let reponse: Enveloppe<ImageBrute> =
        serde_json::from_str(json).map_err(|e| format!("Réponse de SteamGridDB illisible : {e}"))?;
    Ok(reponse
        .data
        .into_iter()
        .map(|i| i.url)
        .find(|u| url_autorisee(u)))
}

/// Seules les images en HTTPS venant de SteamGridDB sont téléchargées.
fn url_autorisee(url: &str) -> bool {
    let Some(reste) = url.strip_prefix("https://") else {
        return false;
    };
    let hote = reste.split(['/', '?', '#']).next().unwrap_or_default();
    hote == "steamgriddb.com" || hote.ends_with(".steamgriddb.com")
}

pub fn rechercher(cle: &str, nom: &str) -> Result<Vec<ResultatJeu>, String> {
    let url = format!("{API}/search/autocomplete/{}", encoder_segment(nom.trim()));
    match appel_api(&url, cle)? {
        Some(json) => analyser_recherche(&json),
        None => Ok(Vec::new()),
    }
}

fn url_images(genre: Genre, id_sgdb: u64) -> Option<String> {
    let filtres = "types=static&mimes=image/png,image/jpeg";
    match genre {
        Genre::Jaquette => Some(format!(
            "{API}/grids/game/{id_sgdb}?dimensions=600x900&{filtres}"
        )),
        Genre::Banniere => Some(format!("{API}/heroes/game/{id_sgdb}?{filtres}")),
        Genre::Icone => None,
    }
}

fn telecharger_genre(
    cle: &str,
    id_sgdb: u64,
    genre: Genre,
    id_jeu: u64,
    dossier: &Path,
) -> Result<Option<String>, String> {
    let Some(api) = url_images(genre, id_sgdb) else {
        return Ok(None);
    };
    let Some(json) = appel_api(&api, cle)? else {
        return Ok(None);
    };
    let Some(url) = premiere_image(&json)? else {
        return Ok(None);
    };
    let octets = reseau::telecharger(&url, TAILLE_IMAGE_MAX)?;
    enregistrer_image(&octets, id_jeu, genre, dossier).map(Some)
}

/// Télécharge les images demandées d'un jeu SteamGridDB. Un genre sans image
/// disponible reste à `None`.
pub fn telecharger_visuels(
    cle: &str,
    id_sgdb: u64,
    id_jeu: u64,
    genres: &[Genre],
    dossier: &Path,
) -> Result<Telecharges, String> {
    let mut resultat = Telecharges::default();
    for &genre in genres {
        let nom = match telecharger_genre(cle, id_sgdb, genre, id_jeu, dossier) {
            Ok(nom) => nom,
            Err(e) => {
                // Pas de fichier orphelin si la seconde image échoue.
                if let Some(f) = resultat.jaquette.iter().chain(&resultat.banniere).next() {
                    let _ = fs::remove_file(dossier.join(f));
                }
                return Err(e);
            }
        };
        match genre {
            Genre::Jaquette => resultat.jaquette = nom,
            Genre::Banniere => resultat.banniere = nom,
            Genre::Icone => {}
        }
    }
    Ok(resultat)
}

/// Vérifie que les octets sont une vraie image et l'écrit dans `dossier`.
/// Le nom du fichier ne dépend que du jeu et du format réel de l'image.
pub fn enregistrer_image(
    octets: &[u8],
    id_jeu: u64,
    genre: Genre,
    dossier: &Path,
) -> Result<String, String> {
    let lecteur = image::ImageReader::new(Cursor::new(octets))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let ext = match lecteur.format() {
        Some(image::ImageFormat::Png) => "png",
        Some(image::ImageFormat::Jpeg) => "jpg",
        Some(image::ImageFormat::WebP) => "webp",
        _ => return Err("Format d'image non supporté (PNG, JPG ou WebP).".into()),
    };
    lecteur
        .decode()
        .map_err(|_| "Impossible de lire cette image.".to_string())?;
    let prefixe = match genre {
        Genre::Banniere => "banniere",
        Genre::Jaquette | Genre::Icone => "jaquette",
    };
    let nom = format!("{id_jeu}_{prefixe}_{}.{ext}", maintenant());
    fs::write(dossier.join(&nom), octets).map_err(|e| e.to_string())?;
    Ok(nom)
}

/// Importe une image choisie par l'utilisateur.
pub fn importer_image(
    source: &Path,
    id_jeu: u64,
    genre: Genre,
    dossier: &Path,
) -> Result<String, String> {
    let taille = fs::metadata(source).map_err(|e| e.to_string())?.len();
    if taille > TAILLE_IMAGE_MAX {
        return Err("Cette image est trop lourde (30 Mo au maximum).".into());
    }
    let octets = fs::read(source).map_err(|e| e.to_string())?;
    enregistrer_image(&octets, id_jeu, genre, dossier)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_1x1() -> Vec<u8> {
        let mut octets = Vec::new();
        image::RgbaImage::new(1, 1)
            .write_to(&mut Cursor::new(&mut octets), image::ImageFormat::Png)
            .unwrap();
        octets
    }

    fn dossier_test(nom: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("omni_test_{nom}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// Interroge le vrai SteamGridDB : `cargo test -- --ignored`.
    #[test]
    #[ignore = "requiert le réseau"]
    fn une_cle_invalide_est_refusee_en_direct() {
        let erreur = rechercher("cle-invalide", "Portal").unwrap_err();
        assert!(erreur.contains("Clé SteamGridDB refusée"), "{erreur}");
    }

    #[test]
    fn encode_accents_espaces_et_barres() {
        assert_eq!(encoder_segment("Half-Life 2"), "Half-Life%202");
        assert_eq!(encoder_segment("a/b?c"), "a%2Fb%3Fc");
        assert_eq!(encoder_segment("Été"), "%C3%89t%C3%A9");
    }

    #[test]
    fn analyse_les_resultats_de_recherche() {
        let json = r#"{"success":true,"data":[
            {"id":12,"name":"Portal","types":["steam"],"verified":true},
            {"id":13,"name":"Portal 2","types":["steam"],"verified":true}]}"#;
        assert_eq!(
            analyser_recherche(json).unwrap(),
            vec![
                ResultatJeu { id: 12, nom: "Portal".into() },
                ResultatJeu { id: 13, nom: "Portal 2".into() },
            ]
        );
        assert!(analyser_recherche(r#"{"success":true,"data":[]}"#).unwrap().is_empty());
        assert!(analyser_recherche("pas du json").is_err());
    }

    #[test]
    fn ignore_les_images_hors_de_steamgriddb() {
        let json = r#"{"data":[
            {"url":"http://cdn2.steamgriddb.com/a.png"},
            {"url":"https://steamgriddb.com.piege.test/b.png"},
            {"url":"https://cdn2.steamgriddb.com/c.png"}]}"#;
        assert_eq!(
            premiere_image(json).unwrap().as_deref(),
            Some("https://cdn2.steamgriddb.com/c.png")
        );
        assert_eq!(premiere_image(r#"{"data":[]}"#).unwrap(), None);
    }

    #[test]
    fn url_autorisee_verifie_l_hote_exact() {
        assert!(url_autorisee("https://cdn2.steamgriddb.com/grid/x.png"));
        assert!(!url_autorisee("https://piegesteamgriddb.com/x.png"));
        assert!(!url_autorisee("https://exemple.test/?h=steamgriddb.com"));
    }

    #[test]
    fn enregistre_une_vraie_image_avec_son_format_reel() {
        let dossier = dossier_test("sgdb_ok");
        let nom = enregistrer_image(&png_1x1(), 7, Genre::Banniere, &dossier).unwrap();
        assert!(nom.starts_with("7_banniere_") && nom.ends_with(".png"), "{nom}");
        assert!(dossier.join(&nom).is_file());
        let _ = fs::remove_dir_all(dossier);
    }

    #[test]
    fn refuse_ce_qui_n_est_pas_une_image() {
        let dossier = dossier_test("sgdb_ko");
        assert!(enregistrer_image(b"<html>", 7, Genre::Jaquette, &dossier).is_err());
        assert_eq!(fs::read_dir(&dossier).unwrap().count(), 0);
        let _ = fs::remove_dir_all(dossier);
    }

    #[test]
    fn la_cle_vide_retire_le_fichier() {
        let dossier = dossier_test("sgdb_cle");
        let d = Dossiers::depuis_racine(dossier.clone());
        assert_eq!(lire_cle(&d), "");
        ecrire_cle(&d, "  abc123\n").unwrap();
        assert_eq!(lire_cle(&d), "abc123");
        ecrire_cle(&d, "").unwrap();
        assert!(!d.cle_api.exists());
        ecrire_cle(&d, "").unwrap();
        let _ = fs::remove_dir_all(dossier);
    }
}
