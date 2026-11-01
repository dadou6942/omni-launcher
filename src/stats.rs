//! Calculs de statistiques de jeu, indépendants de l'interface.

use crate::config::Jeu;
use chrono::{Datelike, Duration as JourDuree, Local, NaiveDate, TimeZone};

/// Durée lisible : « 2 h 05 », « 45 min », « < 1 min ».
pub fn duree(secs: u64) -> String {
    let minutes = secs / 60;
    match (minutes / 60, minutes % 60) {
        (0, 0) if secs > 0 => "< 1 min".into(),
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m:02}"),
    }
}

/// « aujourd'hui », « hier », « il y a 3 jours »… en heure locale.
pub fn il_y_a(ts: u64, maintenant: u64) -> String {
    let (Some(jour), Some(auj)) = (date_locale(ts), date_locale(maintenant)) else {
        return "jamais".into();
    };
    let jours = (auj - jour).num_days();
    match jours {
        ..=0 => "aujourd'hui".into(),
        1 => "hier".into(),
        2..=30 => format!("il y a {jours} jours"),
        31..=364 => format!("il y a {} mois", jours / 30),
        _ => format!("il y a {} an(s)", jours / 365),
    }
}

pub fn date_locale(ts: u64) -> Option<NaiveDate> {
    Local
        .timestamp_opt(ts as i64, 0)
        .single()
        .map(|d| d.date_naive())
}

/// Temps joué dans les sessions commencées depuis `depuis`.
pub fn temps_depuis(jeux: &[Jeu], depuis: u64) -> u64 {
    jeux.iter()
        .flat_map(|j| &j.sessions)
        .filter(|s| s.debut >= depuis)
        .map(|s| s.duree)
        .sum()
}

/// Temps joué par jour sur les `nb_jours` derniers jours (le plus ancien
/// d'abord, aujourd'hui en dernier). Une session compte pour son jour de début.
pub fn activite_par_jour(jeux: &[Jeu], aujourdhui: NaiveDate, nb_jours: usize) -> Vec<(NaiveDate, u64)> {
    let premier = aujourdhui - JourDuree::days(nb_jours as i64 - 1);
    let mut jours: Vec<(NaiveDate, u64)> = (0..nb_jours)
        .map(|i| (premier + JourDuree::days(i as i64), 0))
        .collect();
    for s in jeux.iter().flat_map(|j| &j.sessions) {
        if let Some(d) = date_locale(s.debut)
            && d >= premier
            && d <= aujourdhui
        {
            jours[(d - premier).num_days() as usize].1 += s.duree;
        }
    }
    jours
}

/// Les jeux les plus joués : (id, temps), du plus joué au moins joué.
pub fn plus_joues(jeux: &[Jeu], n: usize) -> Vec<(u64, u64)> {
    let mut v: Vec<(u64, u64)> = jeux
        .iter()
        .filter(|j| j.temps_jeu > 0)
        .map(|j| (j.id, j.temps_jeu))
        .collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v.truncate(n);
    v
}

/// Temps par tag. Un jeu avec plusieurs tags compte dans chacun.
pub fn temps_par_tag(jeux: &[Jeu]) -> Vec<(String, u64)> {
    let mut v: Vec<(String, u64)> = Vec::new();
    for j in jeux.iter().filter(|j| j.temps_jeu > 0) {
        for t in &j.tags {
            match v.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(t)) {
                Some((_, s)) => *s += j.temps_jeu,
                None => v.push((t.clone(), j.temps_jeu)),
            }
        }
    }
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v
}

/// Libellé court d'un jour pour l'axe : « lun. 3 ».
pub fn libelle_jour(d: NaiveDate) -> String {
    let jour = ["lun.", "mar.", "mer.", "jeu.", "ven.", "sam.", "dim."]
        [d.weekday().num_days_from_monday() as usize];
    format!("{jour} {}", d.day())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn jeu(id: u64, sessions: &[(u64, u64)], tags: &[&str]) -> Jeu {
        let mut j = Jeu::nouveau(id, format!("J{id}"), PathBuf::from(format!("C:/{id}.exe")), None);
        for &(d, s) in sessions {
            j.ajouter_session(d, s);
        }
        for t in tags {
            j.ajouter_tag(t);
        }
        j
    }

    #[test]
    fn format_duree() {
        assert_eq!(duree(0), "0 min");
        assert_eq!(duree(30), "< 1 min");
        assert_eq!(duree(45 * 60), "45 min");
        assert_eq!(duree(2 * 3600), "2 h");
        assert_eq!(duree(2 * 3600 + 5 * 60), "2 h 05");
    }

    #[test]
    fn classement_et_tags() {
        let jeux = vec![
            jeu(1, &[(100, 600)], &["Coop"]),
            jeu(2, &[(100, 3600), (200, 60)], &["Coop", "Rétro"]),
            jeu(3, &[], &["Rétro"]),
        ];
        assert_eq!(plus_joues(&jeux, 10), vec![(2, 3660), (1, 600)]);
        assert_eq!(plus_joues(&jeux, 1).len(), 1);
        assert_eq!(
            temps_par_tag(&jeux),
            vec![("Coop".to_string(), 4260), ("Rétro".to_string(), 3660)]
        );
        assert_eq!(temps_depuis(&jeux, 150), 60);
    }

    #[test]
    fn activite_sur_la_periode() {
        let maintenant = Local::now();
        let auj = maintenant.date_naive();
        let ts = maintenant.timestamp() as u64;
        let jeux = vec![jeu(1, &[(ts, 120), (ts - 86400 * 30, 999)], &[])];
        let a = activite_par_jour(&jeux, auj, 14);
        assert_eq!(a.len(), 14);
        assert_eq!(a.last().unwrap(), &(auj, 120));
        assert_eq!(a.iter().map(|(_, s)| s).sum::<u64>(), 120);
    }

    #[test]
    fn il_y_a_relatif() {
        let n = Local::now().timestamp() as u64;
        assert_eq!(il_y_a(n, n), "aujourd'hui");
        assert_eq!(il_y_a(n - 86400 * 5, n), "il y a 5 jours");
    }
}
