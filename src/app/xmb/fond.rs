//! Fond animé en vagues douces, teinté par la couleur du thème.

use eframe::egui::{self, Color32, Mesh, Pos2, Rect};

const COLONNES: usize = 72;

struct Vague {
    /// Position verticale de repos, en fraction de la hauteur.
    base: f32,
    amplitude: f32,
    longueur: f32,
    vitesse: f32,
    epaisseur: f32,
    opacite: f32,
}

const VAGUES: [Vague; 3] = [
    Vague { base: 0.42, amplitude: 0.075, longueur: 1.15, vitesse: 0.16, epaisseur: 0.30, opacite: 0.10 },
    Vague { base: 0.56, amplitude: 0.055, longueur: 0.85, vitesse: -0.11, epaisseur: 0.26, opacite: 0.08 },
    Vague { base: 0.70, amplitude: 0.040, longueur: 1.55, vitesse: 0.08, epaisseur: 0.22, opacite: 0.06 },
];

/// Hauteur de la crête en `x` (0 à 1), au temps `t`, en fraction de la hauteur.
fn crete(v: &Vague, x: f32, t: f32) -> f32 {
    let phase = x * std::f32::consts::TAU / v.longueur + t * v.vitesse * std::f32::consts::TAU;
    v.base + v.amplitude * (phase.sin() * 0.7 + (phase * 0.53 + 1.3).sin() * 0.3)
}

fn teinte(couleur: Color32, opacite: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        couleur.r(),
        couleur.g(),
        couleur.b(),
        (opacite.clamp(0.0, 1.0) * 255.0) as u8,
    )
}

fn ruban(mesh: &mut Mesh, zone: Rect, v: &Vague, couleur: Color32, t: f32) {
    let depart = mesh.vertices.len() as u32;
    for i in 0..=COLONNES {
        let x = i as f32 / COLONNES as f32;
        let px = zone.left() + x * zone.width();
        let haut = zone.top() + crete(v, x, t) * zone.height();
        let bas = haut + v.epaisseur * zone.height();
        // Bord net en haut, fondu vers le bas.
        mesh.colored_vertex(Pos2::new(px, haut), teinte(couleur, v.opacite));
        mesh.colored_vertex(Pos2::new(px, bas), teinte(couleur, 0.0));
    }
    for i in 0..COLONNES as u32 {
        let a = depart + i * 2;
        mesh.add_triangle(a, a + 1, a + 2);
        mesh.add_triangle(a + 1, a + 3, a + 2);
    }
}

/// Peint les vagues. Sans animation, `t` reste fixe et l'image ne change pas.
pub(super) fn peindre(peintre: &egui::Painter, zone: Rect, couleur: Color32, t: f32) {
    let mut mesh = Mesh::default();
    for v in &VAGUES {
        ruban(&mut mesh, zone, v, couleur, t);
    }
    peintre.add(egui::Shape::mesh(mesh));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_cretes_restent_dans_l_ecran() {
        for v in &VAGUES {
            for i in 0..200 {
                let (x, t) = (i as f32 / 200.0, i as f32 * 0.37);
                let c = crete(v, x, t);
                assert!(c > 0.0 && c < 1.0, "{c}");
            }
        }
    }

    #[test]
    fn la_vague_avance_avec_le_temps() {
        assert_ne!(crete(&VAGUES[0], 0.5, 0.0), crete(&VAGUES[0], 0.5, 1.0));
    }
}
