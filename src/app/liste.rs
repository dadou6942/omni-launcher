//! Vue liste de la bibliothèque et section « Joués récemment ».

use super::*;

const HAUTEUR_LIGNE: f32 = 54.0;
const TAILLE_ICONE_LIGNE: f32 = 40.0;
const LARGEUR_STATUT: f32 = 110.0;
const LARGEUR_TEMPS: f32 = 110.0;
const LARGEUR_DERNIER: f32 = 150.0;

const LARGEUR_MIN_RECENTE: f32 = 250.0;
const HAUTEUR_RECENTE: f32 = 92.0;
const ECART_RECENTES: f32 = 14.0;
const RECENTS_MAX: usize = 6;

/// Abscisses (depuis le bord gauche de la ligne) des colonnes affichées.
/// Les moins utiles disparaissent quand la fenêtre rétrécit.
#[derive(Debug, PartialEq)]
struct Colonnes {
    statut: Option<f32>,
    temps: Option<f32>,
    dernier: Option<f32>,
    /// Largeur laissée au nom.
    fin_nom: f32,
}

fn colonnes(largeur: f32) -> Colonnes {
    let mut droite = largeur;
    let mut placer = |seuil: f32, taille: f32| {
        (largeur >= seuil).then(|| {
            droite -= taille;
            droite
        })
    };
    let dernier = placer(760.0, LARGEUR_DERNIER);
    let temps = placer(600.0, LARGEUR_TEMPS);
    let statut = placer(460.0, LARGEUR_STATUT);
    Colonnes {
        statut,
        temps,
        dernier,
        fin_nom: droite,
    }
}

fn texte_coupe(
    ui: &egui::Ui,
    peintre: &egui::Painter,
    pos: egui::Pos2,
    texte: &str,
    largeur: f32,
    police: FontId,
    couleur: Color32,
) {
    let mut job = egui::text::LayoutJob::simple_singleline(texte.to_string(), police, couleur);
    job.wrap.max_width = largeur.max(10.0);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    let galley = ui.painter().layout_job(job);
    peintre.galley(pos, galley, couleur);
}

impl App {
    pub(super) fn liste(&mut self, ui: &mut egui::Ui, ids: &[u64], actions: &mut Vec<Action>) {
        let p = theme::p();
        let largeur = ui.available_width();
        let c = colonnes(largeur);

        // Titres des colonnes.
        let (rect, _) = ui.allocate_exact_size(Vec2::new(largeur, 20.0), Sense::hover());
        let peintre = ui.painter();
        let police = FontId::proportional(13.0);
        let titre = |x: f32, texte: &str| {
            peintre.text(
                egui::pos2(rect.left() + x, rect.center().y),
                Align2::LEFT_CENTER,
                texte,
                police.clone(),
                p.fg_muted,
            );
        };
        titre(HAUTEUR_LIGNE + 8.0, "Nom");
        if let Some(x) = c.statut {
            titre(x, "Statut");
        }
        if let Some(x) = c.temps {
            titre(x, "Temps de jeu");
        }
        if let Some(x) = c.dernier {
            titre(x, "Dernier lancement");
        }
        ui.add_space(2.0);

        let espacement = ui.spacing().item_spacing.y;
        ui.spacing_mut().item_spacing.y = 2.0;
        for &id in ids {
            self.ligne(ui, id, largeur, &c, actions);
        }
        ui.spacing_mut().item_spacing.y = espacement;
        ui.add_space(14.0);
    }

    fn ligne(
        &mut self,
        ui: &mut egui::Ui,
        id: u64,
        largeur: f32,
        c: &Colonnes,
        actions: &mut Vec<Action>,
    ) {
        let Some(jeu) = self.bib.jeu(id).cloned() else {
            return;
        };
        let p = theme::p();
        let manquant = self.bib.manquants.contains(&id);
        let (rect, reponse) =
            ui.allocate_exact_size(Vec2::new(largeur, HAUTEUR_LIGNE), Sense::click());
        if !ui.is_rect_visible(rect) {
            return; // Ligne hors écran : rien à dessiner ni à charger.
        }
        let peintre = ui.painter_at(rect);
        if reponse.hovered() || reponse.context_menu_opened() {
            peintre.rect_filled(rect, CornerRadius::same(8), p.bg_tuile_hover);
        }

        let zone_icone = egui::Rect::from_center_size(
            egui::pos2(rect.left() + HAUTEUR_LIGNE / 2.0, rect.center().y),
            Vec2::splat(TAILLE_ICONE_LIGNE),
        );
        self.dessiner_icone(ui, &jeu, zone_icone, manquant);

        let x_nom = rect.left() + HAUTEUR_LIGNE + 8.0;
        let largeur_nom = rect.left() + c.fin_nom - x_nom - 12.0;
        if self.renommage.as_ref().is_some_and(|r| r.id == id) {
            let zone = egui::Rect::from_min_size(
                egui::pos2(x_nom, rect.top() + 12.0),
                Vec2::new(largeur_nom, 28.0),
            );
            self.champ_renommage(ui, zone, egui::Align2::LEFT_CENTER, actions);
        } else {
            let (y_nom, sous_titre) = if jeu.tags.is_empty() {
                (rect.center().y - 9.0, None)
            } else {
                (rect.top() + 9.0, Some(jeu.tags.join("  ·  ")))
            };
            let police = FontId::new(15.0, theme::gras());
            let couleur = if manquant { p.fg_muted } else { p.fg };
            let pos = egui::pos2(x_nom, y_nom);
            texte_coupe(ui, &peintre, pos, &jeu.nom, largeur_nom, police, couleur);
            if let Some(tags) = sous_titre {
                let pos = egui::pos2(x_nom, y_nom + 20.0);
                let police = FontId::proportional(13.0);
                texte_coupe(ui, &peintre, pos, &tags, largeur_nom, police, p.fg_muted);
            }
        }

        let colonne = |x: f32, texte: &str, couleur: Color32| {
            peintre.text(
                egui::pos2(rect.left() + x, rect.center().y),
                Align2::LEFT_CENTER,
                texte,
                FontId::proportional(14.0),
                couleur,
            );
        };
        if let Some(x) = c.statut {
            if manquant {
                colonne(x, "Introuvable", p.danger);
            } else if jeu.statut != Statut::Aucun {
                colonne(x, jeu.statut.libelle(), couleur_statut(jeu.statut));
            }
        }
        if let Some(x) = c.temps {
            if jeu.temps_jeu > 0 {
                colonne(x, &stats::duree(jeu.temps_jeu), p.fg);
            } else {
                colonne(x, "—", p.fg_muted);
            }
        }
        if let Some(x) = c.dernier {
            match jeu.dernier_lancement {
                Some(t) => colonne(x, &stats::il_y_a(t, maintenant()), p.fg_muted),
                None => colonne(x, "Jamais", p.fg_muted),
            }
        }

        interactions_jeu(reponse, &jeu, actions);
    }

    /// Les derniers jeux lancés, sur une seule rangée.
    pub(super) fn recents(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let largeur = ui.available_width();
        let tenues = ((largeur + ECART_RECENTES) / (LARGEUR_MIN_RECENTE + ECART_RECENTES)).floor();
        let n = (tenues as usize).clamp(1, RECENTS_MAX);
        let ids = self.bib.config.recents(n);
        if ids.is_empty() {
            return;
        }
        titre_section(ui, "Joués récemment");
        let largeur_carte = (largeur - ECART_RECENTES * (n - 1) as f32) / n as f32;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = ECART_RECENTES;
            for id in ids {
                self.carte_recente(ui, id, Vec2::new(largeur_carte, HAUTEUR_RECENTE), actions);
            }
        });
        ui.add_space(18.0);
    }

    fn carte_recente(
        &mut self,
        ui: &mut egui::Ui,
        id: u64,
        taille: Vec2,
        actions: &mut Vec<Action>,
    ) {
        let Some(jeu) = self.bib.jeu(id).cloned() else {
            return;
        };
        let p = theme::p();
        let (rect, reponse) = ui.allocate_exact_size(taille, Sense::click());
        if !ui.is_rect_visible(rect) {
            return;
        }
        let survol = reponse.hovered() || reponse.context_menu_opened();
        let arrondi = CornerRadius::same(10);
        let peintre = ui.painter_at(rect);
        peintre.rect_filled(rect, arrondi, if survol { p.bg_tuile_hover } else { p.bg_menu });

        let banniere = jeu
            .banniere
            .as_deref()
            .and_then(|f| self.icones.visuel(id, Genre::Banniere, f));
        if let Some(t) = banniere {
            let uv = icons::uv_couvrir(t.size_vec2(), rect.size());
            egui::Image::new(&t).uv(uv).corner_radius(arrondi).paint_at(ui, rect);
            // Voile pour garder le texte lisible, quel que soit le thème.
            let opacite = if survol { 130 } else { 175 };
            peintre.rect_filled(rect, arrondi, voile(p.bg_menu, opacite));
        }
        peintre.rect_stroke(rect, arrondi, Stroke::new(1.0, p.bordure), egui::StrokeKind::Inside);

        let marge = (rect.height() - 60.0) / 2.0;
        let zone_icone =
            egui::Rect::from_min_size(rect.min + Vec2::splat(marge), Vec2::splat(60.0));
        self.dessiner_icone(ui, &jeu, zone_icone, self.bib.manquants.contains(&id));

        let x = zone_icone.right() + 12.0;
        let largeur = rect.right() - x - 10.0;
        let y = rect.center().y;
        let police = FontId::new(15.0, theme::gras());
        texte_coupe(ui, &peintre, egui::pos2(x, y - 22.0), &jeu.nom, largeur, police, p.fg);
        let joue = jeu
            .dernier_lancement
            .map(|t| format!("Joué {}", stats::il_y_a(t, maintenant())))
            .unwrap_or_default();
        let police = FontId::proportional(13.0);
        texte_coupe(ui, &peintre, egui::pos2(x, y), &joue, largeur, police, p.fg_muted);

        interactions_jeu(reponse, &jeu, actions);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toutes_les_colonnes_sur_une_fenetre_large() {
        let c = colonnes(1000.0);
        assert_eq!(c.dernier, Some(850.0));
        assert_eq!(c.temps, Some(740.0));
        assert_eq!(c.statut, Some(630.0));
        assert_eq!(c.fin_nom, 630.0);
    }

    #[test]
    fn les_colonnes_disparaissent_quand_la_fenetre_retrecit() {
        let c = colonnes(650.0);
        assert_eq!(c.dernier, None);
        assert_eq!(c.temps, Some(540.0));
        let c = colonnes(400.0);
        assert_eq!((c.statut, c.temps, c.dernier), (None, None, None));
        assert_eq!(c.fin_nom, 400.0);
    }
}
