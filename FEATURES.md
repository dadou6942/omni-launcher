# Omni Launcher : checklist anti-régression

Inventaire de la v2.2.0, dressé pour la refonte de l'interface. Aucune ligne de la partie 1 ne doit disparaître.

- **Bureau** : la fonctionnalité marche à la souris et au clavier dans la nouvelle interface.
- **Manette** : elle marche sans souris dans le mode plein écran.
- `[x]` : essayé dans l'application lancée.
- `[~]` : accessible dans l'interface et couvert par le code, mais pas essayé à l'écran.
- `[ ]` : pas encore accessible.

Aucune manette physique n'a servi aux essais : la colonne Manette a été testée au clavier, qui envoie les mêmes commandes. Le mapping des boutons est vérifié par des tests unitaires.

La colonne « Aujourd'hui » indique où se trouve la fonctionnalité dans la v2.2.0.

## 1. Fonctionnalités existantes

### Bibliothèque et affichage

| Fonctionnalité | Aujourd'hui | Bureau | Manette |
|---|---|---|---|
| Vue icônes | Menu de vue | [x] | n/a |
| Vue jaquettes | Menu de vue | [x] | n/a |
| Vue liste (nom, tags, statut, temps de jeu, dernier lancement) | Menu de vue | [x] | n/a |
| Colonnes de liste masquées quand la fenêtre rétrécit | Vue liste | [~] | n/a |
| Section Favoris en tête de bibliothèque | Bibliothèque | [x] | [x] |
| Section « Joués récemment » | Bibliothèque | [x] | [~] |
| Masquer « Joués récemment » | Paramètres | [~] | [~] |
| Taille des tuiles réglable (90 à 220 px) | Paramètres | [~] | [~] |
| Compteur de jeux | En-tête | [x] | [~] |
| Bibliothèque vide : message et boutons d'ajout | Bibliothèque | [~] | [~] |
| Tri par nom | Menu de tri | [~] | [~] |
| Tri par dernier lancement | Menu de tri | [~] | [~] |
| Tri par date d'ajout | Menu de tri | [~] | [~] |
| Tri par temps de jeu | Menu de tri | [~] | [~] |
| Recherche instantanée sur le nom et les tags | En-tête | [~] | [x] |
| `Ctrl+F` place le curseur dans la recherche | Clavier | [~] | n/a |
| `Entrée` lance le premier résultat | Recherche | [~] | [~] |
| `Échap` efface la recherche | Recherche | [~] | [~] |
| Filtre par tag | Étiquettes | [x] | [x] |
| Filtre par statut | Étiquettes | [~] | [x] |
| Filtre « Favoris » | Plein écran | [~] | [x] |
| Infobulle du jeu (résumé, statut, tags, notes, chemin) | Survol | [~] | [~] |
| Pastille de statut sur la tuile | Tuile | [x] | [x] |
| Jeu introuvable signalé | Tuile, liste | [x] | [x] |

### Ajout de jeux

| Fonctionnalité | Aujourd'hui | Bureau | Manette |
|---|---|---|---|
| Ajouter un ou plusieurs `.exe` | Bouton | [~] | [~] |
| Glisser-déposer de `.exe` | Fenêtre | [~] | n/a |
| Glisser-déposer d'un dossier (lance le scan) | Fenêtre | [~] | n/a |
| Scanner un dossier | Bouton, Paramètres | [~] | [x] |
| Progression du scan et annulation | Fenêtre de scan | [~] | [~] |
| Bibliothèque Steam reconnue (`steamapps\common`) | Scan | [~] | [~] |
| Désinstalleurs, redistribuables et outils de plantage écartés | Scan | [~] | [~] |
| Exe principal pré-coché | Revue du scan | [~] | [x] |
| Revue : tout cocher, tout décocher, uniquement les principaux | Revue du scan | [~] | [x] |
| Revue : corriger les noms avant import | Revue du scan | [~] | [~] |
| Re-scanner le dernier dossier | Paramètres | [~] | [x] |
| Dernier dossier scanné affiché | Paramètres | [~] | [~] |

### Actions sur un jeu

| Fonctionnalité | Aujourd'hui | Bureau | Manette |
|---|---|---|---|
| Lancer (double-clic, UAC géré) | Tuile, menu | [~] | [x] |
| Ouvrir l'emplacement | Menu du jeu | [~] | [~] |
| Favori : ajouter ou retirer | Menu du jeu, Y | [~] | [~] |
| Tags : ajouter, retirer, reprendre un tag existant | Menu du jeu | [~] | [~] |
| Statut et notes | Menu du jeu | [~] | [~] |
| Renommer | Menu du jeu | [~] | [~] |
| Icône personnalisée et retour à l'icône d'origine | Menu du jeu | [~] | [~] |
| Jaquette et bannière | Menu du jeu | [~] | [~] |
| Arguments de lancement | Menu du jeu | [~] | [~] |
| Retirer du launcher, avec confirmation | Menu du jeu | [~] | [~] |
| Supprimer du disque vers la corbeille, avec confirmation | Menu du jeu | [~] | [~] |
| Garde-fous de suppression et message de refus | Menu du jeu | [~] | [~] |
| Jeu introuvable : localiser, retirer ou fermer | Au lancement | [~] | [~] |
| Vérifier les jeux introuvables | Paramètres | [~] | [~] |
| Retirer tous les introuvables, avec confirmation | Paramètres | [~] | [~] |

### Parties et statistiques

| Fonctionnalité | Aujourd'hui | Bureau | Manette |
|---|---|---|---|
| Réduction du launcher pendant une partie | Automatique | [~] | [x] |
| Mesure du temps de jeu, lanceurs intermédiaires compris | Automatique | [~] | [x] |
| Message si le jeu n'est pas détecté sous 90 s | Notification | [~] | [~] |
| Temps enregistré si le launcher ferme en pleine partie | Automatique | [~] | [~] |
| Retour ou fermeture à la fin de la partie (réglable) | Paramètres | [~] | [x] |
| Statistiques : total, 7 jours, parties, jeux joués | Vue Statistiques | [x] | [~] |
| Activité des 14 derniers jours | Vue Statistiques | [x] | [~] |
| Jeux les plus joués | Vue Statistiques | [x] | [~] |
| Temps par tag | Vue Statistiques | [x] | [~] |
| Jeux jamais lancés | Vue Statistiques | [~] | [~] |

### Tags, images et apparence

| Fonctionnalité | Aujourd'hui | Bureau | Manette |
|---|---|---|---|
| Renommer un tag sur tous les jeux | Paramètres | [~] | [~] |
| Supprimer un tag partout | Paramètres | [~] | [~] |
| Icônes extraites des exe, cache disque, chargement en arrière-plan | Automatique | [~] | [~] |
| Vider le cache d'icônes | Paramètres | [~] | [~] |
| Recherche SteamGridDB et choix du bon jeu | Jaquette et bannière | [~] | [~] |
| Images personnelles (PNG, JPG, WebP) | Jaquette et bannière | [~] | [~] |
| Retirer une jaquette ou une bannière | Jaquette et bannière | [~] | [~] |
| Clé SteamGridDB, lien pour en obtenir une | Paramètres | [~] | [~] |
| Récupération groupée avec progression et annulation | Paramètres | [~] | [~] |
| 7 thèmes avec aperçu | Paramètres | [~] | [~] |
| Couleur d'accent libre et retour à celle du thème | Paramètres | [~] | [~] |

### Données, système et mises à jour

| Fonctionnalité | Aujourd'hui | Bureau | Manette |
|---|---|---|---|
| `config.json` unique, écriture atomique | Automatique | [~] | [~] |
| Migration depuis les anciennes versions | Automatique | [x] | [x] |
| Fichier corrompu mis de côté (`.bak`) | Automatique | [~] | [~] |
| Noms d'images importés validés | Automatique | [~] | [~] |
| Mode portable, repli sur `%APPDATA%` | Automatique | [~] | [~] |
| Copie quotidienne, 10 conservées | Automatique | [~] | [~] |
| Copie de sécurité avant remplacement | Automatique | [~] | [~] |
| Restaurer une sauvegarde, avec confirmation | Paramètres | [~] | [~] |
| Exporter la bibliothèque | Paramètres | [~] | [~] |
| Importer : fusionner ou tout remplacer | Paramètres | [~] | [~] |
| Ouvrir le dossier des données et celui des sauvegardes | Paramètres | [~] | [~] |
| Vérification des mises à jour, au démarrage et à la demande | Paramètres | [~] | [~] |
| Bouton « version disponible » | En-tête | [~] | [~] |
| Taille et position de la fenêtre mémorisées | Automatique | [~] | n/a |
| Exe unique, sans console, icône intégrée | Compilation | [x] | [x] |
| Notifications | Bas de fenêtre | [x] | [x] |
| Astuces | Paramètres | [~] | [~] |

### Plein écran actuel

| Fonctionnalité | Aujourd'hui | Bureau | Manette |
|---|---|---|---|
| `F11`, bouton 🎮 et Start | Bascule | [~] | [x] |
| Navigation à la croix et au stick, avec répétition | Plein écran | n/a | [~] |
| Navigation au clavier (flèches, `Entrée`, `Échap`, `F`, `Q`, `E`) | Plein écran | [~] | n/a |
| Commandes de la manette ignorées pendant une partie | Automatique | n/a | [~] |
| Détection de la manette et nom affiché | En-tête | n/a | [x] |
| Horloge | En-tête | n/a | [x] |
| Tuiles portrait dès qu'un jeu a une jaquette | Plein écran | n/a | [x] |
| Bannière du jeu sélectionné | Panneau bas | n/a | [x] |
| Détails : résumé, statut, tags, notes | Panneau bas | n/a | [x] |
| Glyphes Xbox ou clavier selon l'entrée | Barre d'aide | n/a | [x] |

## 2. Nouveautés demandées

### Socle

- [x] Design tokens centralisés dans un seul fichier
- [x] Couche métier séparée de l'interface, appelée par les deux modes
- [x] Animations de 150 à 250 ms, jamais bloquantes
- [x] Fluide au-delà de 500 jeux, listes virtualisées, images paresseuses
- [~] Toute fonctionnalité accessible en 2 actions maximum

### Mode bureau

- [x] Barre latérale repliable : Bibliothèque, Récents, Favoris, Collections, Paramètres
- [x] Recherche toujours visible
- [x] Palette de commandes `Ctrl+K` avec toutes les actions
- [x] Panneau de détail du jeu : lancer, ouvrir le dossier, modifier, retirer
- [~] Doublons signalés dans l'aperçu du scan (aujourd'hui ils sont masqués)
- [x] Raccourcis clavier documentés dans l'interface

### Mode plein écran XMB

- [x] Barre de catégories : Jeux, Récents, Favoris, Collections, Paramètres, Quitter
- [x] Liste verticale pour les menus, carrousel horizontal de jaquettes pour les jeux, élément focalisé agrandi avec nom et infos secondaires
- [x] Fond en vagues lié au thème, désactivable
- [x] Horloge et barre d'aide des boutons
- [~] Mapping : A lancer, B retour, Y options, X recherche, LB/RB catégories, Start paramètres
- [x] Clavier virtuel
- [x] Navigateur de dossiers interne pour scanner à la manette
- [~] Glyphes Xbox et PlayStation
- [~] Paramètres entièrement utilisables à la manette
- [x] Option « démarrer en plein écran »
- [~] Bascule automatique quand une manette est branchée (option)
- [x] Retour au focus sur le jeu quand il se ferme
- [~] Sons de navigation discrets, désactivables

## 3. Limites connues

- **Manette physique** : une manette branchée a été détectée et a affiché les boutons PlayStation automatiquement, mais les parcours complets ont été testés au clavier.
- **Son** : le code est testé, mais personne n'a écouté le résultat.
- **Statistiques en plein écran** : l'activité des 14 derniers jours est un total, sans le graphique du mode bureau.
- **Fenêtres externes** : « Ouvrir l'emplacement », « Ouvrir le dossier » et « Obtenir une clé » ouvrent l'Explorateur ou le navigateur, qui se pilotent à la souris.
- **Mode bureau à la manette** : seul Start est actif, il ouvre le plein écran.
- **SteamGridDB** : le téléchargement avec une vraie clé n'a pas été essayé.
