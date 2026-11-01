//! Vue Statistiques : chiffres clés, activité récente, classements.

use super::*;

const JOURS_ACTIVITE: usize = 14;
const TOP: usize = 10;

impl App {
    pub(super) fn statistiques(&mut self, ui: &mut egui::Ui) {
        let p = theme::p();
        let jeux = &self.config.jeux;
        let total: u64 = jeux.iter().map(|j| j.temps_jeu).sum();
        let maintenant = maintenant();

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.set_max_width(820.0);
                ui.add_space(10.0);
                ui.label(RichText::new("Statistiques de jeu").size(26.0).family(theme::gras()));
                ui.add_space(20.0);

                // Chiffres clés
                let joues = jeux.iter().filter(|j| j.temps_jeu > 0).count();
                let sessions: usize = jeux.iter().map(|j| j.sessions.len()).sum();
                let semaine = stats::temps_depuis(jeux, maintenant.saturating_sub(7 * 86400));
                ui.columns(4, |cols| {
                    tuile_chiffre(&mut cols[0], "Temps de jeu total", &stats::duree(total));
                    tuile_chiffre(&mut cols[1], "7 derniers jours", &stats::duree(semaine));
                    tuile_chiffre(&mut cols[2], "Parties mesurées", &sessions.to_string());
                    tuile_chiffre(&mut cols[3], "Jeux joués", &format!("{joues} / {}", jeux.len()));
                });
                ui.add_space(14.0);

                if total == 0 {
                    carte(ui, "Pas encore de temps de jeu", |ui| {
                        ui.label(
                            RichText::new(
                                "Lancez un jeu depuis le launcher : il se réduit pendant la partie, \
                                 mesure sa durée et l'enregistre à la fin.",
                            )
                            .color(p.fg_muted),
                        );
                    });
                } else {
                    let auj = chrono::Local::now().date_naive();
                    let activite = stats::activite_par_jour(jeux, auj, JOURS_ACTIVITE);
                    carte(ui, &format!("Activité des {JOURS_ACTIVITE} derniers jours"), |ui| {
                        histogramme_jours(ui, &activite);
                    });

                    let top = stats::plus_joues(jeux, TOP);
                    carte(ui, "Les plus joués", |ui| {
                        let max = top.first().map(|t| t.1).unwrap_or(1);
                        for (id, temps) in &top {
                            let nom = jeux.iter().find(|j| j.id == *id).map(|j| j.nom.as_str()).unwrap_or("?");
                            barre(ui, nom, *temps, max, total);
                        }
                    });

                    let par_tag = stats::temps_par_tag(jeux);
                    if !par_tag.is_empty() {
                        carte(ui, "Temps par tag", |ui| {
                            ui.label(
                                RichText::new("Un jeu qui a plusieurs tags compte dans chacun d'eux.")
                                    .small()
                                    .color(p.fg_muted),
                            );
                            let max = par_tag.first().map(|t| t.1).unwrap_or(1);
                            for (tag, temps) in &par_tag {
                                barre(ui, tag, *temps, max, total);
                            }
                        });
                    }
                }

                let jamais: Vec<&str> = jeux
                    .iter()
                    .filter(|j| j.nb_lancements == 0 && j.temps_jeu == 0)
                    .map(|j| j.nom.as_str())
                    .collect();
                if !jamais.is_empty() {
                    carte(ui, &format!("Jamais lancés depuis le launcher ({})", jamais.len()), |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                            for nom in jamais.iter().take(30) {
                                ui.label(
                                    RichText::new(format!(" {nom} "))
                                        .background_color(p.bg_btn)
                                        .color(p.fg),
                                );
                            }
                            if jamais.len() > 30 {
                                ui.label(RichText::new(format!("et {} autres", jamais.len() - 30)).color(p.fg_muted));
                            }
                        });
                    });
                }
            });
        });
    }
}

/// Chiffre clé dans une tuile.
fn tuile_chiffre(ui: &mut egui::Ui, libelle: &str, valeur: &str) {
    let p = theme::p();
    egui::Frame::new()
        .fill(p.bg_menu)
        .stroke(Stroke::new(1.0, p.bordure))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.label(RichText::new(libelle).small().color(p.fg_muted));
                ui.label(RichText::new(valeur).size(24.0).family(theme::gras()).color(p.fg));
            });
        });
}

/// Histogramme vertical d'une seule série : temps joué par jour.
fn histogramme_jours(ui: &mut egui::Ui, jours: &[(chrono::NaiveDate, u64)]) {
    let p = theme::p();
    let hauteur_barres = 140.0;
    let hauteur_axe = 22.0;
    let largeur = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(largeur, hauteur_barres + hauteur_axe + 18.0), Sense::hover());
    let peintre = ui.painter_at(rect);

    let max = jours.iter().map(|j| j.1).max().unwrap_or(0).max(60);
    let haut = rect.top() + 18.0;
    let base = haut + hauteur_barres;

    // Repère discret du maximum, ligne de base.
    peintre.text(
        egui::pos2(rect.left(), rect.top()),
        Align2::LEFT_TOP,
        format!("max {}", stats::duree(max)),
        FontId::proportional(11.5),
        p.fg_muted,
    );
    peintre.line_segment(
        [egui::pos2(rect.left(), haut), egui::pos2(rect.right(), haut)],
        Stroke::new(1.0, p.bordure.linear_multiply(0.5)),
    );
    peintre.line_segment(
        [egui::pos2(rect.left(), base), egui::pos2(rect.right(), base)],
        Stroke::new(1.0, p.bordure),
    );

    let n = jours.len().max(1) as f32;
    let pas = rect.width() / n;
    let largeur_barre = (pas - 2.0).clamp(4.0, 36.0);
    for (i, (date, secs)) in jours.iter().enumerate() {
        let cx = rect.left() + pas * (i as f32 + 0.5);
        let h = if *secs == 0 { 0.0 } else { (*secs as f32 / max as f32 * hauteur_barres).max(3.0) };
        let barre = egui::Rect::from_min_max(
            egui::pos2(cx - largeur_barre / 2.0, base - h),
            egui::pos2(cx + largeur_barre / 2.0, base),
        );
        if h > 0.0 {
            peintre.rect_filled(
                barre,
                CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 },
                p.accent,
            );
        }
        // Libellé d'un jour sur deux si la place manque.
        if pas >= 44.0 || i % 2 == (jours.len() - 1) % 2 {
            peintre.text(
                egui::pos2(cx, base + 4.0),
                Align2::CENTER_TOP,
                stats::libelle_jour(*date),
                FontId::proportional(11.5),
                p.fg_muted,
            );
        }
        // Zone de survol plus grande que la barre.
        let zone = egui::Rect::from_min_max(egui::pos2(cx - pas / 2.0, haut), egui::pos2(cx + pas / 2.0, base));
        let id = ui.scope_id().with(("jour", i));
        let r = ui.interact(zone, id, Sense::hover());
        if r.hovered() {
            peintre.rect_filled(zone, CornerRadius::same(4), p.fg.linear_multiply(0.04));
            r.on_hover_text(format!("{} : {}", stats::libelle_jour(*date), stats::duree(*secs)));
        }
    }
}

/// Ligne de classement : libellé, barre proportionnelle, valeur.
fn barre(ui: &mut egui::Ui, libelle: &str, valeur: u64, max: u64, total: u64) {
    let p = theme::p();
    let largeur_libelle = 200.0;
    let largeur_valeur = 80.0;
    let hauteur = 22.0;
    let (rect, r) = ui.allocate_exact_size(Vec2::new(ui.available_width(), hauteur + 6.0), Sense::hover());
    let peintre = ui.painter_at(rect);

    let galley = ui.painter().layout(
        libelle.to_string(),
        FontId::proportional(14.0),
        p.fg,
        largeur_libelle - 10.0,
    );
    let galley = if galley.rows.len() > 1 {
        let mut job = egui::text::LayoutJob::simple_singleline(libelle.to_string(), FontId::proportional(14.0), p.fg);
        job.wrap.max_width = largeur_libelle - 10.0;
        job.wrap.max_rows = 1;
        job.wrap.overflow_character = Some('…');
        ui.painter().layout_job(job)
    } else {
        galley
    };
    peintre.galley(
        egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.0),
        galley,
        p.fg,
    );

    let zone = egui::Rect::from_min_max(
        egui::pos2(rect.left() + largeur_libelle, rect.center().y - hauteur / 2.0 + 3.0),
        egui::pos2(rect.right() - largeur_valeur, rect.center().y + hauteur / 2.0 - 3.0),
    );
    let fraction = valeur as f32 / max.max(1) as f32;
    let plein = egui::Rect::from_min_size(zone.min, Vec2::new((zone.width() * fraction).max(3.0), zone.height()));
    peintre.rect_filled(
        plein,
        CornerRadius { nw: 0, sw: 0, ne: 4, se: 4 },
        p.accent,
    );
    peintre.text(
        egui::pos2(rect.right(), rect.center().y),
        Align2::RIGHT_CENTER,
        stats::duree(valeur),
        FontId::proportional(14.0),
        p.fg_muted,
    );
    if r.hovered() {
        let part = valeur as f64 / total.max(1) as f64 * 100.0;
        r.on_hover_text(format!("{libelle} : {} ({part:.0} % du temps total)", stats::duree(valeur)));
    }
}
