//! Clavier virtuel : toute saisie de texte reste possible à la manette.

use super::*;

const LIGNES: [&str; 5] = [
    "1234567890",
    "azertyuiop",
    "qsdfghjklm",
    "wxcvbn,.-'",
    "éèêàçùô!?:",
];
/// Dernière rangée : touches spéciales.
const SPECIALES: [Speciale; 4] = [
    Speciale::Majuscule,
    Speciale::Espace,
    Speciale::Effacer,
    Speciale::Valider,
];
const LONGUEUR_MAX: usize = 2000;
const COTE_TOUCHE: f32 = 58.0;
const ECART_TOUCHE: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Speciale {
    Majuscule,
    Espace,
    Effacer,
    Valider,
}

impl Speciale {
    fn libelle(self) -> &'static str {
        match self {
            Speciale::Majuscule => "Maj",
            Speciale::Espace => "Espace",
            Speciale::Effacer => "Effacer",
            Speciale::Valider => "Valider",
        }
    }
}

/// Ce que le clavier demande au reste de l'application.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum IssueClavier {
    Continuer,
    Valider,
    Annuler,
}

pub(super) struct Clavier {
    pub(super) titre: String,
    pub(super) texte: String,
    pub(super) saisie: Saisie,
    masque: bool,
    ligne: usize,
    colonne: usize,
    majuscule: bool,
}

impl Clavier {
    pub(super) fn nouveau(titre: impl Into<String>, texte: String, saisie: Saisie) -> Self {
        let masque = saisie == Saisie::CleApi;
        Self {
            titre: titre.into(),
            texte,
            saisie,
            masque,
            ligne: 1,
            colonne: 0,
            majuscule: false,
        }
    }

    fn colonnes(&self, ligne: usize) -> usize {
        if ligne < LIGNES.len() {
            LIGNES[ligne].chars().count()
        } else {
            SPECIALES.len()
        }
    }

    fn deplacer(&mut self, c: Commande) {
        let lignes = LIGNES.len() + 1;
        match c {
            Commande::Haut => self.ligne = (self.ligne + lignes - 1) % lignes,
            Commande::Bas => self.ligne = (self.ligne + 1) % lignes,
            Commande::Gauche => {
                let n = self.colonnes(self.ligne);
                self.colonne = (self.colonne.min(n - 1) + n - 1) % n;
            }
            Commande::Droite => {
                let n = self.colonnes(self.ligne);
                self.colonne = (self.colonne.min(n - 1) + 1) % n;
            }
            _ => {}
        }
        self.colonne = self.colonne.min(self.colonnes(self.ligne) - 1);
    }

    fn ecrire(&mut self, c: char) {
        if self.texte.chars().count() >= LONGUEUR_MAX {
            return;
        }
        if self.majuscule {
            self.texte.extend(c.to_uppercase());
            self.majuscule = false;
        } else {
            self.texte.push(c);
        }
    }

    fn appuyer(&mut self) -> IssueClavier {
        match LIGNES.get(self.ligne) {
            Some(rangee) => {
                if let Some(c) = rangee.chars().nth(self.colonne) {
                    self.ecrire(c);
                }
            }
            None => match SPECIALES[self.colonne] {
                Speciale::Majuscule => self.majuscule = !self.majuscule,
                Speciale::Espace => self.ecrire(' '),
                Speciale::Effacer => {
                    self.texte.pop();
                }
                Speciale::Valider => return IssueClavier::Valider,
            },
        }
        IssueClavier::Continuer
    }

    pub(super) fn commande(&mut self, c: Commande) -> IssueClavier {
        match c {
            Commande::Haut | Commande::Bas | Commande::Gauche | Commande::Droite => self.deplacer(c),
            Commande::Valider => return self.appuyer(),
            Commande::Retour => return IssueClavier::Annuler,
            Commande::Recherche => {
                self.texte.pop();
            }
            Commande::Options => self.ecrire(' '),
            Commande::FiltrePrecedent | Commande::FiltreSuivant => {
                self.majuscule = !self.majuscule;
            }
            Commande::Parametres => return IssueClavier::Valider,
            Commande::BasculerPleinEcran => {}
        }
        IssueClavier::Continuer
    }

    /// Texte tapé sur un clavier physique, s'il y en a un.
    pub(super) fn recevoir_texte(&mut self, texte: &str) {
        for c in texte.chars().filter(|c| !c.is_control()) {
            self.ecrire(c);
        }
    }

    pub(super) fn dessiner(&self, ui: &mut egui::Ui, zone: egui::Rect) {
        let p = theme::p();
        let peintre = ui.painter_at(zone);
        peintre.rect_filled(zone, CornerRadius::ZERO, voile(p.bg_main, 225));

        let largeur = 10.0 * COTE_TOUCHE + 9.0 * ECART_TOUCHE;
        let x0 = zone.center().x - largeur / 2.0;
        let mut y = zone.top() + zone.height() * 0.16;
        peintre.text(
            egui::pos2(x0, y),
            Align2::LEFT_BOTTOM,
            &self.titre,
            FontId::new(theme::POLICE_XMB, theme::gras()),
            p.fg,
        );
        y += theme::ESPACE_S;

        let champ = egui::Rect::from_min_size(egui::pos2(x0, y), Vec2::new(largeur, 56.0));
        peintre.rect_filled(champ, CornerRadius::same(theme::RAYON_M), p.bg_btn);
        let visible = if self.masque {
            "•".repeat(self.texte.chars().count())
        } else {
            self.texte.replace('\n', " ")
        };
        let fin: String = {
            let n = visible.chars().count();
            visible.chars().skip(n.saturating_sub(58)).collect()
        };
        peintre.text(
            champ.left_center() + Vec2::new(theme::ESPACE_L, 0.0),
            Align2::LEFT_CENTER,
            format!("{fin}|"),
            FontId::proportional(theme::POLICE_XMB),
            p.fg,
        );
        y = champ.bottom() + theme::ESPACE_L;

        for (l, rangee) in LIGNES.iter().enumerate() {
            for (c, touche) in rangee.chars().enumerate() {
                let x = x0 + c as f32 * (COTE_TOUCHE + ECART_TOUCHE);
                let r = egui::Rect::from_min_size(egui::pos2(x, y), Vec2::splat(COTE_TOUCHE));
                let texte: String = if self.majuscule {
                    touche.to_uppercase().collect()
                } else {
                    touche.to_string()
                };
                touche_clavier(&peintre, r, &texte, l == self.ligne && c == self.colonne);
            }
            y += COTE_TOUCHE + ECART_TOUCHE;
        }

        let l_speciale = (largeur - 3.0 * ECART_TOUCHE) / 4.0;
        for (c, s) in SPECIALES.iter().enumerate() {
            let x = x0 + c as f32 * (l_speciale + ECART_TOUCHE);
            let r = egui::Rect::from_min_size(egui::pos2(x, y), Vec2::new(l_speciale, COTE_TOUCHE));
            let actif = self.ligne == LIGNES.len() && c == self.colonne;
            touche_clavier(&peintre, r, s.libelle(), actif);
        }
    }
}

fn touche_clavier(peintre: &egui::Painter, r: egui::Rect, texte: &str, actif: bool) {
    let p = theme::p();
    let (fond, encre) = if actif {
        (p.accent, p.sur_accent())
    } else {
        (p.bg_btn, p.fg)
    };
    peintre.rect_filled(r, CornerRadius::same(theme::RAYON_M), fond);
    peintre.text(
        r.center(),
        Align2::CENTER_CENTER,
        texte,
        FontId::proportional(theme::POLICE_SECTION),
        encre,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clavier() -> Clavier {
        Clavier::nouveau("Test", String::new(), Saisie::Recherche)
    }

    #[test]
    fn taper_une_lettre_et_effacer() {
        let mut c = clavier();
        assert_eq!(c.commande(Commande::Valider), IssueClavier::Continuer);
        assert_eq!(c.texte, "a");
        c.commande(Commande::Droite);
        c.commande(Commande::Valider);
        assert_eq!(c.texte, "az");
        c.commande(Commande::Recherche);
        assert_eq!(c.texte, "a");
        c.commande(Commande::Options);
        assert_eq!(c.texte, "a ");
    }

    #[test]
    fn la_majuscule_ne_vaut_que_pour_une_lettre() {
        let mut c = clavier();
        c.commande(Commande::FiltreSuivant);
        c.commande(Commande::Valider);
        c.commande(Commande::Valider);
        assert_eq!(c.texte, "Aa");
    }

    #[test]
    fn la_navigation_boucle_et_reste_dans_la_rangee() {
        let mut c = clavier();
        c.commande(Commande::Gauche);
        assert_eq!(c.colonne, 9);
        // La rangée spéciale n'a que 4 touches.
        for _ in 0..4 {
            c.commande(Commande::Bas);
        }
        assert_eq!(c.ligne, LIGNES.len());
        assert_eq!(c.colonne, 3);
        assert_eq!(c.commande(Commande::Valider), IssueClavier::Valider);
        c.commande(Commande::Bas);
        assert_eq!(c.ligne, 0);
    }

    #[test]
    fn annuler_et_valider_depuis_les_boutons() {
        let mut c = clavier();
        assert_eq!(c.commande(Commande::Retour), IssueClavier::Annuler);
        assert_eq!(c.commande(Commande::Parametres), IssueClavier::Valider);
    }

    #[test]
    fn clavier_physique_sans_caracteres_de_controle() {
        let mut c = clavier();
        c.recevoir_texte("é\tb");
        assert_eq!(c.texte, "éb");
    }
}
