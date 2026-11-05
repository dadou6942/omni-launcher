//! Vérification des mises à jour : compare la version de l'application à la
//! dernière release publiée sur GitHub. Rien n'est téléchargé ni installé.

use crate::reseau;
use serde::Deserialize;

const API: &str = "https://api.github.com/repos/dadou6942/omni-launcher/releases/latest";
pub const PAGE_RELEASES: &str = "https://github.com/dadou6942/omni-launcher/releases";

pub const VERSION_ACTUELLE: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publication {
    pub version: String,
    pub url: String,
}

#[derive(Deserialize)]
struct ReleaseGithub {
    tag_name: String,
    html_url: String,
}

/// « v2.1.0 » ou « 2.1 » en (majeure, mineure, correctif).
fn analyser_version(texte: &str) -> Option<(u32, u32, u32)> {
    let texte = texte.trim().trim_start_matches(['v', 'V']);
    // Un suffixe (« 2.1.0-beta ») ne compte pas dans la comparaison.
    let nombres = texte.split(['-', '+']).next()?;
    let mut parties = nombres.split('.');
    let majeure = parties.next()?.parse().ok()?;
    let mineure = parties.next().map_or(Some(0), |p| p.parse().ok())?;
    let correctif = parties.next().map_or(Some(0), |p| p.parse().ok())?;
    Some((majeure, mineure, correctif))
}

/// `true` si `tag` désigne une version strictement plus récente que `actuelle`.
fn plus_recente(actuelle: &str, tag: &str) -> bool {
    match (analyser_version(actuelle), analyser_version(tag)) {
        (Some(a), Some(t)) => t > a,
        _ => false,
    }
}

fn comparer(json: &str, actuelle: &str) -> Result<Option<Publication>, String> {
    let release: ReleaseGithub =
        serde_json::from_str(json).map_err(|e| format!("Réponse de GitHub illisible : {e}"))?;
    if !plus_recente(actuelle, &release.tag_name) {
        return Ok(None);
    }
    // L'URL sera ouverte dans le navigateur : on n'accepte que notre dépôt.
    let url = if release.html_url.starts_with(PAGE_RELEASES) {
        release.html_url
    } else {
        PAGE_RELEASES.to_string()
    };
    Ok(Some(Publication {
        version: release.tag_name.trim_start_matches(['v', 'V']).to_string(),
        url,
    }))
}

/// Renvoie la dernière publication si elle est plus récente que l'application.
pub fn verifier() -> Result<Option<Publication>, String> {
    let (statut, texte) = reseau::lire_texte(API, None)?;
    match statut {
        200 => comparer(&texte, VERSION_ACTUELLE),
        404 => Ok(None), // Aucune release publiée.
        403 | 429 => Err("GitHub limite les requêtes, réessayez plus tard.".into()),
        code => Err(format!("GitHub a répondu avec le code {code}.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_les_versions_numeriquement() {
        assert!(plus_recente("2.1.0", "v2.2.0"));
        assert!(plus_recente("2.9.0", "2.10.0"));
        assert!(plus_recente("2.1.0", "v3"));
        assert!(!plus_recente("2.1.0", "v2.1.0"));
        assert!(!plus_recente("2.2.0", "v2.1.9"));
    }

    #[test]
    fn un_tag_illisible_n_annonce_rien() {
        assert!(!plus_recente("2.1.0", "nightly"));
        assert!(!plus_recente("2.1.0", ""));
        assert!(!plus_recente("2.1.0", "v2.x"));
    }

    #[test]
    fn publication_plus_recente_detectee() {
        let json = r#"{"tag_name":"v2.3.0","draft":false,
            "html_url":"https://github.com/dadou6942/omni-launcher/releases/tag/v2.3.0"}"#;
        assert_eq!(
            comparer(json, "2.2.0").unwrap(),
            Some(Publication {
                version: "2.3.0".into(),
                url: "https://github.com/dadou6942/omni-launcher/releases/tag/v2.3.0".into(),
            })
        );
        assert_eq!(comparer(json, "2.3.0").unwrap(), None);
    }

    #[test]
    fn url_etrangere_remplacee_par_la_page_du_depot() {
        let json = r#"{"tag_name":"v9.0.0","html_url":"https://exemple.test/piege"}"#;
        assert_eq!(comparer(json, "2.2.0").unwrap().unwrap().url, PAGE_RELEASES);
    }

    /// Interroge le vrai GitHub : `cargo test -- --ignored`.
    #[test]
    #[ignore = "requiert le réseau"]
    fn verification_en_direct() {
        verifier().unwrap();
    }

    #[test]
    fn json_invalide_donne_une_erreur() {
        assert!(comparer("<html>", "2.2.0").is_err());
    }
}
