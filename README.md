# 🎮 Omni Launcher

Un lanceur de jeux léger pour Windows : vos `.exe` (téléchargés _évidemment_ en toute légalité) dans une grille d'icônes, à un double-clic de distance.

La version 2 est une réécriture complète en **Rust** avec [egui](https://github.com/emilk/egui) : un seul exécutable autonome, sans Python ni dépendance à installer, qui démarre quasi instantanément.

![Rust](https://img.shields.io/badge/Rust-2024-orange?logo=rust)
![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%2F%2011-lightgrey?logo=windows)
![Version](https://img.shields.io/badge/version-2.0.0-blue)

---

## Table des matières

- [Fonctionnalités](#fonctionnalités)
- [Installation](#installation)
- [Compiler depuis les sources](#compiler-depuis-les-sources)
- [Utilisation](#utilisation)
- [Données](#données)
- [Structure du projet](#structure-du-projet)
- [Nouveautés de la v2](#nouveautés-de-la-v2)
- [Tests](#tests)

---

## Fonctionnalités

- **Scan de dossier** : parcourt un dossier de jeux (un sous-dossier par jeu) et propose les exécutables trouvés. Le plus probable est pré-coché ; désinstalleurs, redistribuables et outils de rapport de plantage sont écartés automatiquement.
- **Bibliothèques Steam reconnues** : le scan descend directement dans `steamapps\common`.
- **Re-scan** : ajoute en un clic les jeux installés depuis le dernier scan.
- **Glisser-déposer** de `.exe` ou d'un dossier de jeux directement dans la fenêtre.
- **Icônes extraites nativement** des exécutables (API Windows, 256 px), mises en cache et chargées en arrière-plan.
- **Favoris**, affichés en tête de la bibliothèque.
- **Recherche instantanée** et **tri** par nom, dernier lancement ou date d'ajout.
- **Thème sombre** et **taille des tuiles réglable**.
- **Menu contextuel** (clic droit) : favori, renommer, icône personnalisée, arguments de lancement, ouvrir l'emplacement, retirer, supprimer du disque.
- **Jeux introuvables signalés**, avec choix de les relocaliser ou de les retirer.
- **Suppression sécurisée** : le dossier du jeu part à la corbeille (récupérable), avec des garde-fous.
- **Option « fermer après lancement »**, et mémorisation de la taille et de la position de la fenêtre.
- **Mode portable** : toutes les données restent à côté de l'exécutable.

---

## Installation

Téléchargez `omni_launcher.exe` depuis la page [Releases](https://github.com/dadou6942/omni-launcher/releases) et lancez-le. Aucune installation n'est nécessaire : l'exécutable est autonome et peut être placé n'importe où (clé USB comprise).

---

## Compiler depuis les sources

### Prérequis

- [Rust](https://rustup.rs) installé avec `rustup` (édition 2024, donc Rust 1.85 ou plus récent)
- Les [Build Tools de Visual Studio](https://visualstudio.microsoft.com/fr/visual-cpp-build-tools/) avec le composant **« Développement Desktop en C++ »**

### Compilation

```powershell
git clone https://github.com/dadou6942/omni-launcher.git
cd omni-launcher
cargo build --release
```

L'exécutable se trouve dans `target\release\omni_launcher.exe`.

Le fichier `omni_launcher.ico` à la racine du projet est automatiquement intégré à l'exécutable (icône dans l'Explorateur) et à la fenêtre. Sans lui, la compilation fonctionne quand même.

---

## Utilisation

| Action | Geste |
|---|---|
| Lancer un jeu | Double-clic sur sa tuile |
| Ouvrir le menu d'un jeu | Clic droit sur sa tuile |
| Ajouter un jeu | Bouton **+ Ajouter un jeu**, ou glisser-déposer un `.exe` |
| Importer un dossier de jeux | **Scanner un dossier**, ou glisser-déposer le dossier |
| Ajouter les jeux installés depuis | **Paramètres** → **Re-scanner** |
| Rechercher | `Ctrl+F` |
| Lancer le premier résultat | `Entrée` dans la recherche |
| Effacer la recherche | `Échap` |
| Changer l'icône d'un jeu | Clic droit → *Modifier l'icône…* |
| Retirer un jeu (sans toucher au disque) | Clic droit → *Retirer du launcher* |
| Supprimer un jeu du disque | Clic droit → *Supprimer le jeu du disque* |

### Garde-fous de suppression

« Supprimer le jeu du disque » envoie le dossier du jeu à la corbeille. La suppression est refusée s'il s'agit d'un dossier racine, du dossier utilisateur, de `Program Files`, du dossier scanné lui-même ou d'un dossier partagé par plusieurs jeux.

---

## Données

Tout est rangé dans `donnees\` à côté de l'exécutable (mode portable), ou dans `%APPDATA%\OmniLauncher` si ce dossier n'est pas accessible en écriture :

```
donnees/
├── config.json       # Bibliothèque et réglages (écrit de façon atomique)
├── icones/           # Icônes personnalisées
└── cache_icones/     # Icônes extraites des exécutables, en PNG
```

Le dossier est accessible depuis **Paramètres** → **Données** → **Ouvrir le dossier**.

---

## Structure du projet

```
omni-launcher/
├── Cargo.toml
├── build.rs              # Intègre omni_launcher.ico à l'exécutable
├── omni_launcher.ico     # Icône de l'application
└── src/
    ├── main.rs           # Point d'entrée, fenêtre
    ├── app.rs            # Interface : grille, menus, dialogues, paramètres
    ├── config.rs         # Configuration JSON persistante
    ├── scan.rs           # Scan de dossiers et détection de l'exe principal
    ├── icons.rs          # Extraction, cache et chargement des icônes
    ├── platform.rs       # Appels Windows (lancement, icônes, corbeille)
    └── theme.rs          # Thème visuel
```

---

## Nouveautés de la v2

La v1 (Python / Tkinter) reste disponible via le tag [`v1.1.0`](https://github.com/dadou6942/omni-launcher/tree/v1.1.0).

| v1 · Python / Tkinter | v2 · Rust / egui |
|---|---|
| Python et trois paquets à installer | Un seul `.exe` autonome |
| Un raccourci `.lnk` par jeu, relu via COM à chaque démarrage | Un seul `config.json`, écrit de façon atomique |
| Icônes ré-extraites et redimensionnées à chaque rafraîchissement | Extraites une fois, mises en cache, chargées en arrière-plan |
| Toute la grille détruite et recréée à chaque action | Rendu immédiat : seules les tuiles visibles sont dessinées |
| Ajout des jeux un par un | Scan de dossier et bibliothèques Steam |
| Raccourci supprimé automatiquement si le jeu est introuvable | Jeu affiché « Introuvable », à relocaliser ou retirer |
| Suppression définitive du dossier | Envoi à la corbeille, avec garde-fous |
| Renommage = renommer des fichiers sur disque | Renommage = simple changement de nom dans la config |

---

## Tests

```powershell
cargo test
```

Les tests couvrent la configuration (aller-retour JSON, fichier corrompu), le scan (filtrage, choix de l'exe principal, bibliothèque Steam) et le traitement des icônes.
