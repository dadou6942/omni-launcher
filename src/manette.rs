//! Lecture de la manette dans un thread dédié : les boutons sont traduits en
//! commandes de navigation, avec répétition quand une direction est maintenue.

use eframe::egui;
use gilrs::{Axis, Button, EventType, Gilrs};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

const DELAI_REPETITION: Duration = Duration::from_millis(400);
const INTERVALLE_REPETITION: Duration = Duration::from_millis(110);
const SEUIL_STICK: f32 = 0.6;
/// Seuil plus bas pour relâcher : évite les à-coups autour du seuil.
const SEUIL_RELACHE: f32 = 0.4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Commande {
    Haut,
    Bas,
    Gauche,
    Droite,
    /// A (Xbox) / Croix (PlayStation)
    Valider,
    /// B / Rond
    Retour,
    /// Y / Triangle
    Favori,
    /// LB / L1
    FiltrePrecedent,
    /// RB / R1
    FiltreSuivant,
    /// Start / Menu / Options
    BasculerPleinEcran,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MsgManette {
    Commande(Commande),
    Connectee(String),
    Deconnectee,
}

/// Démarre la lecture de la manette. Si aucune API de manette n'est
/// disponible, le canal reste simplement muet.
pub fn demarrer(ctx: egui::Context) -> Receiver<MsgManette> {
    let (tx, rx) = channel();
    let _ = std::thread::Builder::new()
        .name("manette".into())
        .spawn(move || boucle(&tx, &ctx));
    rx
}

fn boucle(tx: &Sender<MsgManette>, ctx: &egui::Context) {
    let Ok(mut gilrs) = Gilrs::new() else { return };
    let envoyer = |m: MsgManette| {
        let ok = tx.send(m).is_ok();
        ctx.request_repaint();
        ok
    };
    for (_, g) in gilrs.gamepads() {
        envoyer(MsgManette::Connectee(g.name().to_string()));
    }

    let mut stick = (0.0f32, 0.0f32);
    let mut croix: Option<Commande> = None;
    let mut repetition = Repetition::default();

    loop {
        let attente = repetition
            .prochaine()
            .map(|t| t.saturating_duration_since(Instant::now()))
            .unwrap_or(Duration::from_millis(500));

        if let Some(ev) = gilrs.next_event_blocking(Some(attente)) {
            let msg = match ev.event {
                EventType::ButtonPressed(b, _) => match b {
                    Button::DPadUp => {
                        croix = Some(Commande::Haut);
                        None
                    }
                    Button::DPadDown => {
                        croix = Some(Commande::Bas);
                        None
                    }
                    Button::DPadLeft => {
                        croix = Some(Commande::Gauche);
                        None
                    }
                    Button::DPadRight => {
                        croix = Some(Commande::Droite);
                        None
                    }
                    autre => bouton(autre).map(MsgManette::Commande),
                },
                EventType::ButtonReleased(b, _) => {
                    if croix.is_some() && croix == direction_croix(b) {
                        croix = None;
                    }
                    None
                }
                EventType::AxisChanged(Axis::LeftStickX, v, _) => {
                    stick.0 = v;
                    None
                }
                EventType::AxisChanged(Axis::LeftStickY, v, _) => {
                    stick.1 = v;
                    None
                }
                EventType::Connected => Some(MsgManette::Connectee(
                    gilrs.gamepad(ev.id).name().to_string(),
                )),
                EventType::Disconnected => {
                    croix = None;
                    stick = (0.0, 0.0);
                    let reste = gilrs.gamepads().count() > 0;
                    (!reste).then_some(MsgManette::Deconnectee)
                }
                _ => None,
            };
            if let Some(m) = msg
                && !envoyer(m)
            {
                return;
            }
        }

        let voulue = croix.or_else(|| direction_stick(stick, repetition.tenue()));
        if let Some(c) = repetition.maj(voulue, Instant::now())
            && !envoyer(MsgManette::Commande(c))
        {
            return;
        }
    }
}

fn bouton(b: Button) -> Option<Commande> {
    Some(match b {
        Button::South => Commande::Valider,
        Button::East => Commande::Retour,
        Button::North => Commande::Favori,
        Button::LeftTrigger => Commande::FiltrePrecedent,
        Button::RightTrigger => Commande::FiltreSuivant,
        Button::Start => Commande::BasculerPleinEcran,
        _ => return None,
    })
}

fn direction_croix(b: Button) -> Option<Commande> {
    Some(match b {
        Button::DPadUp => Commande::Haut,
        Button::DPadDown => Commande::Bas,
        Button::DPadLeft => Commande::Gauche,
        Button::DPadRight => Commande::Droite,
        _ => return None,
    })
}

/// Direction indiquée par le stick gauche (Y positif = haut).
fn direction_stick((x, y): (f32, f32), actuelle: Option<Commande>) -> Option<Commande> {
    let seuil = if actuelle.is_some() {
        SEUIL_RELACHE
    } else {
        SEUIL_STICK
    };
    if x.abs().max(y.abs()) < seuil {
        return None;
    }
    Some(if x.abs() > y.abs() {
        if x > 0.0 { Commande::Droite } else { Commande::Gauche }
    } else if y > 0.0 {
        Commande::Haut
    } else {
        Commande::Bas
    })
}

/// Répétition automatique d'une direction maintenue.
#[derive(Default)]
struct Repetition {
    tenue: Option<(Commande, Instant)>,
}

impl Repetition {
    fn tenue(&self) -> Option<Commande> {
        self.tenue.map(|(c, _)| c)
    }

    fn prochaine(&self) -> Option<Instant> {
        self.tenue.map(|(_, t)| t)
    }

    /// Renvoie la commande à émettre maintenant, s'il y en a une.
    fn maj(&mut self, voulue: Option<Commande>, maintenant: Instant) -> Option<Commande> {
        match (voulue, self.tenue) {
            (None, _) => {
                self.tenue = None;
                None
            }
            (Some(v), Some((c, t))) if v == c => {
                if maintenant >= t {
                    self.tenue = Some((c, maintenant + INTERVALLE_REPETITION));
                    Some(c)
                } else {
                    None
                }
            }
            (Some(v), _) => {
                self.tenue = Some((v, maintenant + DELAI_REPETITION));
                Some(v)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stick_avec_hysteresis() {
        assert_eq!(direction_stick((0.5, 0.0), None), None);
        assert_eq!(direction_stick((0.7, 0.2), None), Some(Commande::Droite));
        assert_eq!(direction_stick((0.1, -0.9), None), Some(Commande::Bas));
        assert_eq!(direction_stick((0.0, 0.8), None), Some(Commande::Haut));
        // Une fois engagé, on tient jusqu'au seuil de relâche.
        assert_eq!(
            direction_stick((0.5, 0.0), Some(Commande::Droite)),
            Some(Commande::Droite)
        );
        assert_eq!(direction_stick((0.3, 0.0), Some(Commande::Droite)), None);
    }

    #[test]
    fn repetition_apres_delai() {
        let t0 = Instant::now();
        let mut r = Repetition::default();
        assert_eq!(r.maj(Some(Commande::Bas), t0), Some(Commande::Bas));
        assert_eq!(r.maj(Some(Commande::Bas), t0 + Duration::from_millis(100)), None);
        let t1 = t0 + DELAI_REPETITION;
        assert_eq!(r.maj(Some(Commande::Bas), t1), Some(Commande::Bas));
        assert_eq!(r.maj(Some(Commande::Bas), t1 + Duration::from_millis(50)), None);
        assert_eq!(
            r.maj(Some(Commande::Bas), t1 + INTERVALLE_REPETITION),
            Some(Commande::Bas)
        );
        // Changement de direction : émission immédiate.
        assert_eq!(
            r.maj(Some(Commande::Gauche), t1 + INTERVALLE_REPETITION),
            Some(Commande::Gauche)
        );
        assert_eq!(r.maj(None, t1 + INTERVALLE_REPETITION), None);
        assert_eq!(r.prochaine(), None);
    }
}
