//! Requêtes HTTP bloquantes, à n'appeler que depuis un thread de travail.

use std::time::Duration;

const DELAI_MAX: Duration = Duration::from_secs(30);
const TAILLE_MAX_TEXTE: u64 = 4 * 1024 * 1024;

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(DELAI_MAX))
        .http_status_as_error(false)
        .user_agent(concat!("OmniLauncher/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// GET textuel. Renvoie le statut HTTP et le corps de la réponse.
pub fn lire_texte(url: &str, jeton: Option<&str>) -> Result<(u16, String), String> {
    let mut requete = agent().get(url);
    if let Some(j) = jeton {
        requete = requete.header("Authorization", format!("Bearer {j}"));
    }
    let mut reponse = requete
        .call()
        .map_err(|e| format!("Connexion impossible : {e}"))?;
    let statut = reponse.status().as_u16();
    let texte = reponse
        .body_mut()
        .with_config()
        .limit(TAILLE_MAX_TEXTE)
        .read_to_string()
        .map_err(|e| format!("Réponse illisible : {e}"))?;
    Ok((statut, texte))
}

/// Télécharge un fichier d'au plus `taille_max` octets.
pub fn telecharger(url: &str, taille_max: u64) -> Result<Vec<u8>, String> {
    let mut reponse = agent()
        .get(url)
        .call()
        .map_err(|e| format!("Connexion impossible : {e}"))?;
    if !reponse.status().is_success() {
        return Err(format!("Téléchargement refusé (code {}).", reponse.status().as_u16()));
    }
    reponse
        .body_mut()
        .with_config()
        .limit(taille_max)
        .read_to_vec()
        .map_err(|e| format!("Téléchargement interrompu : {e}"))
}
