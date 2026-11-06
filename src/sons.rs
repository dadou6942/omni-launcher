//! Sons de navigation du mode plein écran. Ils sont générés au premier usage :
//! rien à embarquer dans l'exécutable, aucune dépendance audio.

use crate::platform;
use std::sync::OnceLock;

const FREQUENCE_ECHANTILLON: u32 = 22_050;
/// Volume crête, volontairement bas : ces sons doivent rester discrets.
const VOLUME: f32 = 0.11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Son {
    Deplacement,
    Validation,
    Retour,
}

/// Un WAV PCM 16 bits mono : une note qui s'éteint rapidement.
fn note(frequence: f32, duree_ms: u32) -> Vec<u8> {
    let n = FREQUENCE_ECHANTILLON * duree_ms / 1000;
    let mut donnees = Vec::with_capacity(n as usize * 2);
    for i in 0..n {
        let t = i as f32 / FREQUENCE_ECHANTILLON as f32;
        let enveloppe = (1.0 - i as f32 / n as f32).powi(2);
        let echantillon = (t * frequence * std::f32::consts::TAU).sin() * enveloppe * VOLUME;
        donnees.extend_from_slice(&((echantillon * i16::MAX as f32) as i16).to_le_bytes());
    }

    let mut wav = Vec::with_capacity(44 + donnees.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + donnees.len() as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&FREQUENCE_ECHANTILLON.to_le_bytes());
    wav.extend_from_slice(&(FREQUENCE_ECHANTILLON * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(donnees.len() as u32).to_le_bytes());
    wav.extend_from_slice(&donnees);
    wav
}

fn wav(son: Son) -> &'static [u8] {
    static SONS: OnceLock<[Vec<u8>; 3]> = OnceLock::new();
    let sons = SONS.get_or_init(|| [note(880.0, 40), note(1320.0, 70), note(520.0, 70)]);
    match son {
        Son::Deplacement => &sons[0],
        Son::Validation => &sons[1],
        Son::Retour => &sons[2],
    }
}

pub fn jouer(son: Son) {
    platform::jouer_wav(wav(son));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_wav_valide_et_sans_saturation() {
        let w = note(440.0, 50);
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(&w[8..12], b"WAVE");
        let taille = u32::from_le_bytes([w[40], w[41], w[42], w[43]]) as usize;
        assert_eq!(taille + 44, w.len());
        let crete = w[44..]
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]).unsigned_abs())
            .max()
            .unwrap();
        assert!(crete <= (i16::MAX as f32 * VOLUME) as u16);
        assert!(crete > 0);
    }

    #[test]
    fn chaque_son_est_different() {
        assert_ne!(wav(Son::Deplacement), wav(Son::Validation));
        assert_ne!(wav(Son::Validation), wav(Son::Retour));
    }
}
