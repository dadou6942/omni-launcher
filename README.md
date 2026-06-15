# 🎮 Omni Launcher

Un launcher visuel de fichiers `.exe` téléchargés totalement légalement, construit en Python.

![Python](https://img.shields.io/badge/Python-3.10%2B-blue?logo=python)
![Platform](https://img.shields.io/badge/Platform-Windows-lightgrey?logo=windows)

---

## Table des matières

- [Principe](#principe)
- [Fonctionnalités](#fonctionnalités)
- [Prérequis](#prérequis)
- [Installation](#installation)
- [Lancement](#lancement)
- [Compiler en `.exe`](#compiler-en-exe)
- [Structure des fichiers](#structure-des-fichiers)
- [Utilisation](#utilisation)

---

## Principe

Omni Launcher repose sur un dossier central contenant des raccourcis Windows (`.lnk`) pointant vers les exécutables de vos jeux. Au démarrage, le launcher :

1. Scanne ce dossier à la recherche de fichiers `.lnk`
2. Vérifie que chaque cible existe toujours sur le disque (nettoyage automatique des raccourcis cassés)
3. Extrait automatiquement l'icône de chaque exécutable si aucune image n'est pas déjà en cache
4. Affiche tous les jeux dans une grille avec leur icône et leur nom

Le dossier de bibliothèque est configurable depuis l'interface. Vous pouvez donc pointer vers un dossier existant contenant déjà vos raccourcis — ils seront importés automatiquement.

---

## Fonctionnalités

- **Grille responsive** — la disposition s'adapte automatiquement à la taille et à l'état (normal/maximisé) de la fenêtre
- **Extraction automatique d'icônes** — les icônes sont extraites directement depuis les `.exe`
- **Ajout de jeu en un clic** — sélectionnez un `.exe`, le raccourci et l'icône sont créés automatiquement
- **Menu contextuel** (clic droit sur un jeu) :
  - Ouvrir le dossier d'installation
  - Renommer le jeu
  - Retirer du launcher (supprime le raccourci, conserve le jeu)
  - Supprimer définitivement (supprime le dossier d'installation complet)
- **Paramètres** — changement du dossier de bibliothèque à la volée
- **Nettoyage automatique** — les raccourcis dont la cible est introuvable sont supprimés au démarrage
- **Images personnalisées** — placez un `.png` ou `.jpg` du nom du jeu dans le sous-dossier `images/` pour remplacer l'icône extraite

---

## Prérequis

- Windows 10 ou 11
- Python 3.10 ou supérieur
- Les dépendances suivantes :

| Package | Usage |
|---|---|
| `Pillow` | Chargement et redimensionnement des images |
| `pywin32` | Création et lecture des raccourcis `.lnk` |
| `icoextract` | Extraction des icônes depuis les `.exe` |

---

## Installation

### 1. Cloner le dépôt

```bash
git clone https://github.com/dadou6942/omni-launcher.git
cd omni-launcher
```

### 2. Créer un environnement virtuel (recommandé)

```bash
python -m venv venv
venv\Scripts\activate
```

### 3. Installer les dépendances

```bash
pip install Pillow pywin32 icoextract
```

> Si `pywin32` nécessite une étape supplémentaire après installation, exécutez :
> ```bash
> python Scripts/pywin32_postinstall.py -install
> ```

---

## Lancement

```bash
python omni_launcher.py
```

Au premier démarrage, un dossier `RACCOURCIS_JEUX/` est créé automatiquement à côté du script. Vous pouvez changer son emplacement depuis **⚙ Paramètres**.

---

## Compiler en `.exe`

Pour distribuer Omni Launcher sans nécessiter Python, utilisez PyInstaller.

### 1. Installer PyInstaller

```bash
pip install pyinstaller
```

### 2. Générer l'exécutable

```bash
pyinstaller --noconfirm --clean --noconsole --onedir --icon=omni_launcher.ico omni_launcher.py
```

L'exécutable se trouvera dans `dist/omni_launcher/omni_launcher.exe`.

---

## Structure des fichiers

```
omni-launcher/
│
├── omni_launcher.py          # Script principal
├── omni_launcher.ico         # Icône de l'application
├── config.json               # Généré automatiquement, contient le chemin du dossier
│
└── RACCOURCIS_JEUX/          # Dossier de bibliothèque (configurable)
    ├── MonJeu.lnk
    ├── AutreJeu.lnk
    └── images/               # Cache des icônes extraites
        ├── MonJeu.ico
        └── AutreJeu.ico
```

Pour utiliser des **images personnalisées** plutôt que les icônes extraites, déposez un fichier `.png` ou `.jpg` portant exactement le même nom que le raccourci dans le dossier `images/` :

```
images/
└── MonJeu.png    ← sera utilisé à la place de MonJeu.ico
```

---

## Utilisation

| Action | Geste |
|---|---|
| Lancer un jeu | Double-clic sur le jeu |
| Ouvrir le menu contextuel | Clic droit sur le jeu |
| Ajouter un jeu | Bouton **Ajouter un jeu** |
| Renommer un jeu | Clic droit → *Renommer le jeu* |
| Retirer du launcher | Clic droit → *Retirer du launcher* |
| Supprimer le jeu du disque | Clic droit → *Supprimer définitivement le jeu* |
| Changer le dossier de bibliothèque | Bouton **Paramètres** |
