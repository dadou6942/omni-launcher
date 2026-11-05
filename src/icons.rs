//! Chargement des icônes en arrière-plan avec cache disque.
//!
//! - L'icône d'un exe est extraite une seule fois puis mise en cache en PNG
//!   (clé = chemin + date de modification + taille du fichier).
//! - Le décodage se fait dans des threads de travail : l'interface ne gèle
//!   jamais, les tuiles affichent une lettre en attendant.

use crate::config::{Dossiers, cle_chemin};
use crate::platform;
use eframe::egui;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

pub const TAILLE_ICONE: u32 = 256;
/// Tailles maximales en mémoire : la moitié de 600x900 et de 1920x620.
const JAQUETTE_MAX: (u32, u32) = (300, 450);
const BANNIERE_MAX: (u32, u32) = (960, 310);

/// Les différentes images d'un jeu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Genre {
    Icone,
    Jaquette,
    Banniere,
}

const GENRES: [Genre; 3] = [Genre::Icone, Genre::Jaquette, Genre::Banniere];

type Cle = (u64, Genre);

#[derive(Clone)]
enum Source {
    Perso(PathBuf),
    Exe(PathBuf),
    Visuel(PathBuf, (u32, u32)),
}

struct Demande {
    cle: Cle,
    generation: u32,
    source: Source,
}

struct Reponse {
    cle: Cle,
    generation: u32,
    image: Option<egui::ColorImage>,
}

enum Etat {
    EnCours,
    Pret(egui::TextureHandle),
    Echec,
}

pub struct ChargeurIcones {
    envoi: Sender<Demande>,
    reception: Receiver<Reponse>,
    etats: HashMap<Cle, Etat>,
    generations: HashMap<Cle, u32>,
    dossier_icones: PathBuf,
    dossier_jaquettes: PathBuf,
}

impl ChargeurIcones {
    pub fn nouveau(ctx: &egui::Context, dossiers: &Dossiers) -> Self {
        let (envoi, rx_demandes) = channel::<Demande>();
        let (tx_reponses, reception) = channel::<Reponse>();
        let rx_demandes = Arc::new(Mutex::new(rx_demandes));

        let nb_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(2)
            .clamp(1, 4);

        for i in 0..nb_threads {
            let rx = Arc::clone(&rx_demandes);
            let tx = tx_reponses.clone();
            let ctx = ctx.clone();
            let cache = dossiers.cache.clone();
            std::thread::Builder::new()
                .name(format!("icones-{i}"))
                .spawn(move || {
                    loop {
                        // Le verrou n'est tenu que le temps de recevoir.
                        let demande = match rx.lock() {
                            Ok(r) => r.recv(),
                            Err(_) => return,
                        };
                        let Ok(d) = demande else { return };
                        let image = match &d.source {
                            Source::Perso(p) => charger_image_fichier(p),
                            Source::Exe(p) => charger_icone_exe(p, &cache),
                            Source::Visuel(p, max) => decoder_image(p, *max),
                        };
                        if tx
                            .send(Reponse {
                                cle: d.cle,
                                generation: d.generation,
                                image,
                            })
                            .is_err()
                        {
                            return;
                        }
                        ctx.request_repaint();
                    }
                })
                .expect("thread icônes");
        }

        Self {
            envoi,
            reception,
            etats: HashMap::new(),
            generations: HashMap::new(),
            dossier_icones: dossiers.icones.clone(),
            dossier_jaquettes: dossiers.jaquettes.clone(),
        }
    }

    /// Récupère les icônes décodées et les envoie au GPU. À appeler à chaque frame.
    pub fn recevoir(&mut self, ctx: &egui::Context) {
        while let Ok(r) = self.reception.try_recv() {
            if self.generations.get(&r.cle).copied().unwrap_or(0) != r.generation {
                continue; // Réponse obsolète (image changée entre-temps).
            }
            let etat = match r.image {
                Some(img) => Etat::Pret(ctx.load_texture(
                    format!("image-{}-{:?}-{}", r.cle.0, r.cle.1, r.generation),
                    img,
                    egui::TextureOptions::LINEAR,
                )),
                None => Etat::Echec,
            };
            self.etats.insert(r.cle, etat);
        }
    }

    /// Renvoie la texture si elle est prête, sinon lance son chargement.
    pub fn texture(
        &mut self,
        id: u64,
        exe: &Path,
        icone_perso: Option<&str>,
    ) -> Option<egui::TextureHandle> {
        let source = match icone_perso {
            Some(nom) => Source::Perso(self.dossier_icones.join(nom)),
            None => Source::Exe(exe.to_path_buf()),
        };
        self.obtenir((id, Genre::Icone), source)
    }

    /// Texture d'une jaquette ou d'une bannière, comme [`Self::texture`].
    pub fn visuel(&mut self, id: u64, genre: Genre, fichier: &str) -> Option<egui::TextureHandle> {
        let max = match genre {
            Genre::Banniere => BANNIERE_MAX,
            Genre::Jaquette | Genre::Icone => JAQUETTE_MAX,
        };
        let source = Source::Visuel(self.dossier_jaquettes.join(fichier), max);
        self.obtenir((id, genre), source)
    }

    fn obtenir(&mut self, cle: Cle, source: Source) -> Option<egui::TextureHandle> {
        match self.etats.get(&cle) {
            Some(Etat::Pret(t)) => return Some(t.clone()),
            Some(Etat::EnCours) | Some(Etat::Echec) => return None,
            None => {}
        }
        let generation = self.generations.get(&cle).copied().unwrap_or(0);
        self.etats.insert(cle, Etat::EnCours);
        let _ = self.envoi.send(Demande {
            cle,
            generation,
            source,
        });
        None
    }

    /// Force le rechargement d'une image (après modification).
    pub fn invalider_genre(&mut self, id: u64, genre: Genre) {
        let cle = (id, genre);
        self.etats.remove(&cle);
        *self.generations.entry(cle).or_insert(0) += 1;
    }

    /// Force le rechargement de toutes les images d'un jeu.
    pub fn invalider(&mut self, id: u64) {
        for genre in GENRES {
            self.invalider_genre(id, genre);
        }
    }

    pub fn tout_invalider(&mut self) {
        let cles: Vec<Cle> = self.etats.keys().copied().collect();
        for (id, genre) in cles {
            self.invalider_genre(id, genre);
        }
    }
}

/// Coordonnées de texture qui remplissent une zone sans déformer l'image :
/// ce qui dépasse est coupé de part et d'autre.
pub fn uv_couvrir(image: egui::Vec2, zone: egui::Vec2) -> egui::Rect {
    let plein = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    if image.x <= 0.0 || image.y <= 0.0 || zone.x <= 0.0 || zone.y <= 0.0 {
        return plein;
    }
    let (ratio_image, ratio_zone) = (image.x / image.y, zone.x / zone.y);
    let centre = egui::pos2(0.5, 0.5);
    if ratio_image > ratio_zone {
        egui::Rect::from_center_size(centre, egui::vec2(ratio_zone / ratio_image, 1.0))
    } else {
        egui::Rect::from_center_size(centre, egui::vec2(1.0, ratio_image / ratio_zone))
    }
}

/// Hachage FNV-1a 64 bits : stable d'une version de Rust à l'autre,
/// contrairement à `DefaultHasher`.
fn fnv1a(donnees: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in donnees {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn cle_cache(exe: &Path) -> Option<String> {
    let meta = std::fs::metadata(exe).ok()?;
    let modif = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let texte = format!("{}|{}|{}", cle_chemin(exe), modif, meta.len());
    Some(format!("{:016x}", fnv1a(texte.as_bytes())))
}

fn charger_icone_exe(exe: &Path, dossier_cache: &Path) -> Option<egui::ColorImage> {
    let cle = cle_cache(exe)?;
    let fichier_cache = dossier_cache.join(format!("{cle}.png"));

    if let Some(img) = charger_image_fichier(&fichier_cache) {
        return Some(img);
    }

    let brute = platform::extraire_icone(exe, TAILLE_ICONE)?;
    let rgba = image::RgbaImage::from_raw(brute.largeur, brute.hauteur, brute.pixels)?;
    let rgba = rogner_transparence(rgba);
    let _ = rgba.save(&fichier_cache);
    Some(vers_color_image(rgba))
}

/// Décode un PNG / JPG / ICO / BMP / WebP. Pour un ICO, `image` choisit
/// automatiquement la plus grande résolution disponible.
fn charger_image_fichier(chemin: &Path) -> Option<egui::ColorImage> {
    decoder_image(chemin, (TAILLE_ICONE, TAILLE_ICONE))
}

fn decoder_image(chemin: &Path, (largeur, hauteur): (u32, u32)) -> Option<egui::ColorImage> {
    let img = image::ImageReader::open(chemin)
        .ok()?
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;
    let img = if img.width() > largeur || img.height() > hauteur {
        img.resize(largeur, hauteur, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };
    Some(vers_color_image(img.to_rgba8()))
}

/// Les icônes 256 px agrandies depuis une petite source ont souvent une
/// marge transparente : on la retire pour que l'icône remplisse la tuile.
fn rogner_transparence(img: image::RgbaImage) -> image::RgbaImage {
    let (l, h) = img.dimensions();
    let (mut x0, mut y0, mut x1, mut y1) = (l, h, 0, 0);
    for (x, y, p) in img.enumerate_pixels() {
        if p[3] > 8 {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    if x1 < x0 || y1 < y0 {
        return img;
    }
    // On garde un carré centré pour ne pas déformer l'icône.
    let cote = (x1 - x0 + 1).max(y1 - y0 + 1);
    if cote >= l.min(h) {
        return img;
    }
    let cx = (x0 + x1).div_ceil(2);
    let cy = (y0 + y1).div_ceil(2);
    let x = cx.saturating_sub(cote / 2).min(l - cote);
    let y = cy.saturating_sub(cote / 2).min(h - cote);
    image::imageops::crop_imm(&img, x, y, cote, cote).to_image()
}

fn vers_color_image(img: image::RgbaImage) -> egui::ColorImage {
    let taille = [img.width() as usize, img.height() as usize];
    egui::ColorImage::from_rgba_unmultiplied(taille, img.as_raw())
}

/// Copie une image choisie par l'utilisateur dans le dossier `icones/`
/// et renvoie le nom du fichier créé.
pub fn importer_icone(source: &Path, id: u64, dossiers: &Dossiers) -> Result<String, String> {
    let ext = source
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let ext = if ext == "jpeg" {
        "jpg".to_string()
    } else {
        ext
    };
    if !matches!(ext.as_str(), "png" | "jpg" | "ico" | "bmp" | "webp") {
        return Err(format!(
            "Format .{ext} non supporté (PNG, JPG, ICO, BMP ou WebP)."
        ));
    }
    // Vérifie que l'image est lisible avant de l'adopter.
    if charger_image_fichier(source).is_none() {
        return Err("Impossible de lire cette image.".into());
    }
    let nom = format!("{id}_{}.{ext}", crate::config::maintenant());
    std::fs::copy(source, dossiers.icones.join(&nom)).map_err(|e| e.to_string())?;
    Ok(nom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rognage_centre() {
        let mut img = image::RgbaImage::new(256, 256);
        for y in 100..156 {
            for x in 90..170 {
                img.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
            }
        }
        let r = rogner_transparence(img);
        assert_eq!(r.dimensions(), (80, 80));
    }

    #[test]
    fn uv_coupe_les_cotes_d_une_image_trop_large() {
        let uv = uv_couvrir(egui::vec2(200.0, 100.0), egui::vec2(100.0, 100.0));
        assert_eq!((uv.left(), uv.right()), (0.25, 0.75));
        assert_eq!((uv.top(), uv.bottom()), (0.0, 1.0));
    }

    #[test]
    fn uv_coupe_le_haut_d_une_image_trop_haute() {
        let uv = uv_couvrir(egui::vec2(100.0, 200.0), egui::vec2(100.0, 100.0));
        assert_eq!((uv.left(), uv.right()), (0.0, 1.0));
        assert_eq!((uv.top(), uv.bottom()), (0.25, 0.75));
    }

    #[test]
    fn uv_entier_si_taille_nulle() {
        let uv = uv_couvrir(egui::vec2(0.0, 0.0), egui::vec2(100.0, 100.0));
        assert_eq!(uv.size(), egui::vec2(1.0, 1.0));
    }

    #[test]
    fn fnv_stable() {
        assert_eq!(fnv1a(b"omni"), fnv1a(b"omni"));
        assert_ne!(fnv1a(b"a"), fnv1a(b"b"));
    }
}
