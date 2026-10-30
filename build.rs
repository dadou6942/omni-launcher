// Si `omni_launcher.ico` est présent à la racine du projet :
// - il devient l'icône de l'exécutable dans l'Explorateur (Windows) ;
// - il est intégré au binaire pour l'icône de la fenêtre.
// Sans ce fichier, la compilation fonctionne normalement.
fn main() {
    println!("cargo:rerun-if-changed=omni_launcher.ico");
    println!("cargo::rustc-check-cfg=cfg(icone_integree)");

    if !std::path::Path::new("omni_launcher.ico").exists() {
        return;
    }
    println!("cargo:rustc-cfg=icone_integree");

    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("omni_launcher.ico");
        if let Err(e) = res.compile() {
            println!("cargo:warning=Icône non intégrée à l'exécutable : {e}");
        }
    }
}
