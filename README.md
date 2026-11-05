# 🎮 Omni Launcher

Un lanceur de jeux léger pour Windows : vos `.exe` (téléchargés _évidemment_ en toute légalité) dans une grille d'icônes

La version 2 est une réécriture complète en **Rust** avec [egui](https://github.com/emilk/egui) : un seul exécutable autonome, sans Python ni dépendance à installer
![Rust](https://img.shields.io/badge/Rust-2024-orange?logo=rust)
![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%2F%2011-lightgrey?logo=windows)
![Version](https://img.shields.io/badge/version-2.2.0-blue)

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
- **Trois vues** : icônes, jaquettes ou liste détaillée (nom, statut, temps de jeu, dernier lancement).
- **Jaquettes et bannières** : téléchargées depuis [SteamGridDB](https://www.steamgriddb.com) avec votre clé d'API gratuite, jeu par jeu ou pour toute la bibliothèque, ou choisies parmi vos images.
- **Section « Joués récemment »** en tête de la bibliothèque, désactivable.
- **Favoris**, affichés en tête de la bibliothèque.
- **Tags** (Coop, Rétro, À finir…) : plusieurs par jeu, filtres en un clic sous l'en-tête, renommage et suppression depuis les paramètres.
- **Statut et notes par jeu** : à jouer, en cours, terminé, abandonné, avec filtres en un clic et notes libres.
- **Temps de jeu** : pendant une partie, le launcher se réduit et surveille les processus du jeu (y compris quand l'exe n'est qu'un petit lanceur), puis enregistre la durée et revient ou se ferme.
- **Statistiques** : temps total, 7 derniers jours, activité des 14 derniers jours, jeux les plus joués, temps par tag, jeux jamais lancés.
- **Mode plein écran à la manette** (F11, bouton 🎮 ou Start) : grandes tuiles, navigation à la croix ou au stick, filtres avec LB/RB. Fonctionne aussi au clavier.
- **Recherche instantanée** (nom ou tag) et **tri** par nom, dernier lancement, date d'ajout ou temps de jeu.
- **7 thèmes de couleurs** (Omni, OLED noir, Nord, Dracula, Catppuccin, Synthwave, Clair), **couleur d'accent libre** et **taille des tuiles réglable**.
- **Vérification des mises à jour** : le launcher signale une nouvelle version publiée sur GitHub. Il ne télécharge et n'installe rien de lui-même, et l'option peut être désactivée.
- **Sauvegardes** : une copie automatique par jour (10 conservées), restauration en un clic, export et import de la bibliothèque.
- **Menu contextuel** (clic droit) : favori, tags, statut et notes, renommer, icône personnalisée, jaquette et bannière, arguments de lancement, ouvrir l'emplacement, retirer, supprimer du disque.
- **Jeux introuvables signalés**, avec choix de les relocaliser ou de les retirer.
- **Suppression sécurisée** : le dossier du jeu part à la corbeille (récupérable), avec des garde-fous.
- **Option « fermer à la fin de la partie »**, et mémorisation de la taille et de la position de la fenêtre.
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
| Importer un dossier de jeux | Bouton **Scanner**, ou glisser-déposer le dossier |
| Ajouter les jeux installés depuis | **Paramètres** → **Re-scanner** |
| Rechercher | `Ctrl+F` |
| Lancer le premier résultat | `Entrée` dans la recherche |
| Effacer la recherche | `Échap` |
| Changer l'icône d'un jeu | Clic droit → *Modifier l'icône…* |
| Ajouter des tags à un jeu | Clic droit → *Tags…* |
| Donner un statut ou écrire des notes | Clic droit → *Statut et notes…* |
| Filtrer par statut ou par tag | Clic sur une étiquette sous l'en-tête |
| Changer de vue | Menu à côté du tri : **Icônes**, **Jaquettes** ou **Liste** |
| Choisir la jaquette et la bannière d'un jeu | Clic droit → *Jaquette et bannière…* |
| Récupérer toutes les images manquantes | **Paramètres** → **Jaquettes et bannières** |
| Vérifier les mises à jour | **Paramètres** → **Mises à jour** |
| Voir les statistiques | Bouton **📊** |
| Mode plein écran | `F11`, bouton **🎮**, ou **Start** sur la manette |
| Changer de thème | **Paramètres** → **Apparence** |
| Restaurer / exporter / importer | **Paramètres** → **Sauvegardes** |
| Retirer un jeu (sans toucher au disque) | Clic droit → *Retirer du launcher* |
| Supprimer un jeu du disque | Clic droit → *Supprimer le jeu du disque* |

### Mode plein écran

| Action | Manette | Clavier |
|---|---|---|
| Se déplacer | Croix ou stick gauche | Flèches |
| Lancer | A | `Entrée` |
| Favori | Y | `F` |
| Filtre précédent / suivant | LB / RB | `Q` / `E` |
| Quitter le plein écran | B ou Start | `Échap` ou `F11` |

### Jaquettes et bannières

Pour les télécharger, créez un compte sur [SteamGridDB](https://www.steamgriddb.com), récupérez votre [clé d'API](https://www.steamgriddb.com/profile/preferences/api) et collez-la dans **Paramètres** → **Jaquettes et bannières**. Sans clé, vous pouvez toujours choisir vos propres images (PNG, JPG ou WebP).

La récupération groupée prend le premier résultat de chaque jeu. Si une image ne correspond pas, corrigez-la par clic droit → *Jaquette et bannière…*, où vous choisissez vous-même le bon jeu.

La clé est enregistrée en clair dans `donnees\cle_steamgriddb.txt`, hors de `config.json` : elle ne figure ni dans les sauvegardes ni dans les exports. Pensez-y si vous copiez ce dossier, par exemple sur une clé USB partagée.

En mode plein écran, les tuiles passent au format portrait dès qu'un jeu a une jaquette, et la bannière du jeu sélectionné s'affiche en bas.

### Temps de jeu

Au lancement d'un jeu, le launcher se réduit et repère les processus dont l'exécutable se trouve dans le dossier du jeu. Les processus qui tournaient déjà avant (un client comme Steam, par exemple) ne comptent pas. À la fin de la partie, la durée est enregistrée, puis le launcher se ferme ou réapparaît selon le réglage. Si le jeu n'apparaît pas dans les 90 secondes (jeu lancé par un autre client), le temps n'est pas mesuré.

### Garde-fous de suppression

« Supprimer le jeu du disque » envoie le dossier du jeu à la corbeille. La suppression est refusée s'il s'agit d'un dossier racine, du dossier utilisateur, de `Program Files`, du dossier scanné lui-même ou d'un dossier partagé par plusieurs jeux.

---

## Données

Tout est rangé dans `donnees\` à côté de l'exécutable (mode portable), ou dans `%APPDATA%\OmniLauncher` si ce dossier n'est pas accessible en écriture :

```
donnees/
├── config.json       # Bibliothèque, temps de jeu et réglages (écrit de façon atomique)
├── cle_steamgriddb.txt  # Clé d'API SteamGridDB, si vous en avez ajouté une
├── icones/           # Icônes personnalisées
├── jaquettes/        # Jaquettes et bannières
├── sauvegardes/      # Copies quotidiennes de config.json (10 conservées)
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
    ├── app.rs            # Interface : grille, menus, dialogues
    ├── app/
    │   ├── liste.rs      # Vue liste, section « Joués récemment »
    │   ├── mises_a_jour.rs # Affichage des mises à jour
    │   ├── parametres.rs # Vue Paramètres
    │   ├── statistiques.rs # Vue Statistiques
    │   ├── tv.rs         # Mode plein écran à la manette
    │   └── visuels.rs    # Jaquettes et bannières
    ├── config.rs         # Configuration JSON persistante
    ├── sauvegarde.rs     # Copies quotidiennes, export, import
    ├── partie.rs         # Suivi des parties (temps de jeu)
    ├── maj.rs            # Recherche de nouvelles versions sur GitHub
    ├── manette.rs        # Lecture de la manette (gilrs)
    ├── reseau.rs         # Requêtes HTTP
    ├── steamgriddb.rs    # API SteamGridDB, enregistrement des images
    ├── stats.rs          # Calculs de statistiques
    ├── scan.rs           # Scan de dossiers et détection de l'exe principal
    ├── icons.rs          # Extraction, cache et chargement des images
    ├── platform.rs       # Appels Windows (lancement, icônes, processus, corbeille)
    └── theme.rs          # Thèmes de couleurs
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

Les tests couvrent la configuration (aller-retour JSON, fichier corrompu, migration depuis les anciennes versions, tags, statuts, jeux récents, fusion), les sauvegardes, le suivi des parties, la manette (seuils du stick, répétition), la navigation du mode plein écran, les statistiques, les thèmes, le scan (filtrage, choix de l'exe principal, bibliothèque Steam), le traitement des images, l'analyse des réponses SteamGridDB et GitHub, et la comparaison des versions.

Deux tests interrogent les vrais services et sont ignorés par défaut :

```powershell
cargo test -- --ignored
```
