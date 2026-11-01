//! Fonctions dépendantes du système : lancement, explorateur, icônes d'exe.
//! Sous Windows on passe par l'API native (plus de pywin32 ni de .lnk) ;
//! ailleurs, des équivalents simples permettent de tester l'interface.

use std::path::Path;

/// Image RGBA brute (largeur, hauteur, pixels).
pub struct ImageRgba {
    pub largeur: u32,
    pub hauteur: u32,
    pub pixels: Vec<u8>,
}

/// Envoie un fichier ou un dossier à la corbeille (récupérable).
pub fn mettre_a_la_corbeille(chemin: &Path) -> Result<(), String> {
    trash::delete(chemin).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod imp {
    use super::ImageRgba;
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
        DeleteObject, GetDIBits, GetObjectW, HBITMAP, HDC,
    };
    use windows::Win32::UI::Shell::{SHDefExtractIconW, ShellExecuteW};
    use windows::Win32::UI::WindowsAndMessaging::{
        DestroyIcon, GetIconInfo, HICON, ICONINFO, SW_SHOWNORMAL,
    };
    use windows::core::{HSTRING, w};

    /// Lance l'exécutable comme un double-clic dans l'Explorateur :
    /// gère l'élévation UAC et le dossier de travail.
    pub fn lancer(exe: &Path, arguments: &str) -> Result<(), String> {
        let dossier = exe.parent().unwrap_or(exe);
        let exe_h = HSTRING::from(exe.as_os_str());
        let dossier_h = HSTRING::from(dossier.as_os_str());
        let args_h = HSTRING::from(arguments);

        let resultat =
            unsafe { ShellExecuteW(None, w!("open"), &exe_h, &args_h, &dossier_h, SW_SHOWNORMAL) };
        let code = resultat.0 as isize;
        if code > 32 {
            return Ok(());
        }
        Err(match code {
            2 | 3 => "Fichier introuvable.".to_string(),
            5 => "Accès refusé (ou élévation annulée).".to_string(),
            8 => "Mémoire insuffisante.".to_string(),
            11 => "Ce fichier n'est pas un exécutable valide.".to_string(),
            _ => format!("Erreur de lancement (code {code})."),
        })
    }

    /// Ouvre l'Explorateur avec l'exe sélectionné.
    pub fn ouvrir_dans_explorateur(chemin: &Path) -> Result<(), String> {
        std::process::Command::new("explorer.exe")
            .raw_arg(format!("/select,\"{}\"", chemin.display()))
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn ouvrir_dossier(dossier: &Path) -> Result<(), String> {
        std::process::Command::new("explorer.exe")
            .arg(dossier)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Extrait l'icône principale d'un exe en haute définition (jusqu'à 256 px).
    pub fn extraire_icone(exe: &Path, taille: u32) -> Option<ImageRgba> {
        let chemin = HSTRING::from(exe.as_os_str());
        let mut hicon = HICON::default();
        unsafe {
            SHDefExtractIconW(&chemin, 0, 0, Some(&mut hicon), None, taille & 0xFFFF)
                .ok()
                .ok()?;
        }
        if hicon.is_invalid() {
            return None;
        }
        let image = unsafe { hicon_vers_rgba(hicon) };
        unsafe {
            let _ = DestroyIcon(hicon);
        }
        image
    }

    unsafe fn hicon_vers_rgba(hicon: HICON) -> Option<ImageRgba> {
        let mut info = ICONINFO::default();
        unsafe { GetIconInfo(hicon, &mut info).ok()? };

        let dc = unsafe { CreateCompatibleDC(None) };
        let resultat = (|| {
            if info.hbmColor.is_invalid() {
                return None;
            }
            let (l, h) = unsafe { dimensions(info.hbmColor)? };
            let mut pixels = unsafe { lire_bits(dc, info.hbmColor, l, h)? };

            // BGRA → RGBA
            for px in pixels.chunks_exact_mut(4) {
                px.swap(0, 2);
            }

            // Anciennes icônes sans canal alpha : on se sert du masque.
            if pixels.chunks_exact(4).all(|px| px[3] == 0) {
                if let Some(masque) = unsafe { lire_bits(dc, info.hbmMask, l, h) } {
                    for (px, m) in pixels.chunks_exact_mut(4).zip(masque.chunks_exact(4)) {
                        px[3] = if m[0] == 0 { 255 } else { 0 };
                    }
                } else {
                    for px in pixels.chunks_exact_mut(4) {
                        px[3] = 255;
                    }
                }
            }
            Some(ImageRgba {
                largeur: l,
                hauteur: h,
                pixels,
            })
        })();

        unsafe {
            let _ = DeleteDC(dc);
            if !info.hbmColor.is_invalid() {
                let _ = DeleteObject(info.hbmColor.into());
            }
            if !info.hbmMask.is_invalid() {
                let _ = DeleteObject(info.hbmMask.into());
            }
        }
        resultat
    }

    unsafe fn dimensions(bmp: HBITMAP) -> Option<(u32, u32)> {
        let mut b = BITMAP::default();
        let n = unsafe {
            GetObjectW(
                bmp.into(),
                std::mem::size_of::<BITMAP>() as i32,
                Some(&mut b as *mut BITMAP as *mut _),
            )
        };
        if n == 0 || b.bmWidth <= 0 || b.bmHeight <= 0 {
            return None;
        }
        Some((b.bmWidth as u32, b.bmHeight as u32))
    }

    /// Lit un bitmap en 32 bits, lignes de haut en bas.
    unsafe fn lire_bits(dc: HDC, bmp: HBITMAP, l: u32, h: u32) -> Option<Vec<u8>> {
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: l as i32,
                biHeight: -(h as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0u8; (l * h * 4) as usize];
        let lignes = unsafe {
            GetDIBits(
                dc,
                bmp,
                0,
                h,
                Some(pixels.as_mut_ptr() as *mut _),
                &mut bmi,
                DIB_RGB_COLORS,
            )
        };
        (lignes as u32 == h).then_some(pixels)
    }

    /// (PID, chemin complet) des processus en cours d'exécution, hors celui-ci.
    /// Les processus protégés (système, anti-triche…) sont simplement ignorés.
    pub fn processus_actifs() -> Vec<(u32, PathBuf)> {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        };

        let moi = std::process::id();
        let mut chemins = Vec::new();
        let Ok(instantane) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
            return chemins;
        };
        let mut entree = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if unsafe { Process32FirstW(instantane, &mut entree) }.is_ok() {
            loop {
                let pid = entree.th32ProcessID;
                if pid != 0
                    && pid != moi
                    && let Some(c) = chemin_processus(pid)
                {
                    chemins.push((pid, c));
                }
                if unsafe { Process32NextW(instantane, &mut entree) }.is_err() {
                    break;
                }
            }
        }
        unsafe {
            let _ = CloseHandle(instantane);
        }
        chemins
    }

    fn chemin_processus(pid: u32) -> Option<PathBuf> {
        use std::os::windows::ffi::OsStringExt;
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{
            OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
            QueryFullProcessImageNameW,
        };
        use windows::core::PWSTR;

        let processus = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        let mut tampon = [0u16; 1024];
        let mut taille = tampon.len() as u32;
        let ok = unsafe {
            QueryFullProcessImageNameW(
                processus,
                PROCESS_NAME_WIN32,
                PWSTR(tampon.as_mut_ptr()),
                &mut taille,
            )
        }
        .is_ok();
        unsafe {
            let _ = CloseHandle(processus);
        }
        ok.then(|| PathBuf::from(std::ffi::OsString::from_wide(&tampon[..taille as usize])))
    }
}

// ---------------------------------------------------------------------------
// Autres systèmes (développement / tests)
// ---------------------------------------------------------------------------

#[cfg(not(windows))]
mod imp {
    use super::ImageRgba;
    use std::path::Path;

    pub fn lancer(exe: &Path, arguments: &str) -> Result<(), String> {
        std::process::Command::new(exe)
            .args(arguments.split_whitespace())
            .current_dir(exe.parent().unwrap_or(Path::new(".")))
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn ouvrir_dans_explorateur(chemin: &Path) -> Result<(), String> {
        ouvrir_dossier(chemin.parent().unwrap_or(chemin))
    }

    pub fn ouvrir_dossier(dossier: &Path) -> Result<(), String> {
        std::process::Command::new("xdg-open")
            .arg(dossier)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn extraire_icone(_exe: &Path, _taille: u32) -> Option<ImageRgba> {
        None
    }

    /// Sous Linux, via `/proc` ; ailleurs, liste vide.
    pub fn processus_actifs() -> Vec<(u32, std::path::PathBuf)> {
        let moi = std::process::id();
        let Ok(entrees) = std::fs::read_dir("/proc") else {
            return Vec::new();
        };
        entrees
            .flatten()
            .filter_map(|e| {
                let pid: u32 = e.file_name().to_string_lossy().parse().ok()?;
                (pid != moi).then_some(())?;
                Some((pid, std::fs::read_link(e.path().join("exe")).ok()?))
            })
            .collect()
    }
}

pub use imp::{extraire_icone, lancer, ouvrir_dans_explorateur, ouvrir_dossier, processus_actifs};

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn processus_actifs_contient_cargo() {
        let liste = super::processus_actifs();
        assert!(liste.iter().all(|(_, p)| p.is_absolute()));
        assert!(
            liste.iter().any(|(_, p)| p
                .file_name()
                .is_some_and(|n| n.eq_ignore_ascii_case("cargo.exe"))),
            "cargo.exe devrait être visible pendant les tests"
        );
    }
}
