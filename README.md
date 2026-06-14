# Omni Launcher 🚀

**Omni Launcher** est un gestionnaire et lanceur de d'applications léger, développé en Python avec une interface graphique Tkinter. 
Il permet de centraliser tous vos jeux téléchargés complètement légalement au même endroit sous forme d'une bibliothèque visuelle.

---

## 🌟 Fonctionnalités

- **Grille Responsive :** L'interface ajuste automatiquement le nombre de colonnes et la taille des icônes en fonction de la taille de la fenêtre (optimisé pour le mode fenêtré et le plein écran).
- **Extraction Auto des Icônes :** Le launcher extrait automatiquement l'icône haute résolution directement depuis le fichier `.exe` du jeu.
- **Édition Inline (Sans Fenêtre) :** Faites un clic droit pour renommer instantanément un jeu directement sur l'interface, sans pop-up intrusive.
- **Gestion Intelligente de la Bibliothèque :** - *Ouvrir le dossier :* Accès direct aux fichiers du jeu.
  - *Retirer du launcher :* Supprime le visuel de l'application sans toucher au jeu.
  - *Supprimer définitivement :* Désinstalle complètement le dossier racine du jeu de votre disque dur (avec confirmation de sécurité).

---

## 📋 Prérequis

Le projet étant fourni sous forme de script Python source, vous devez installer Python 3.x sur votre machine ainsi que les dépendances nécessaires.

Ouvrez votre terminal et installez les bibliothèques requises :

```bash
pip install Pillow pywin32 icoextract pyinstaller
```

---

## ⚙️ Configuration & Lancement (Depuis les sources)

1. **Ajuster le chemin :** Avant de lancer le script, ouvrez `omni_launcher.py` et modifiez la variable `DOSSIER_JEUX` pour pointer vers le dossier où vous souhaitez que le launcher stocke ses raccourcis et ses images :
   ```python
   DOSSIER_JEUX = r"D:\Games\RACCOURCIS_JEUX"
   ```
2. **Lancer le script :**
   ```bash
   python omni_launcher.py
   ```

---

## 📦 Générer l'application (Compilation finale)

Pour une utilisation quotidienne avec un lancement instantané et une stabilité maximale, compilez le projet sous forme de dossier autonome (`--onedir`).

1. Placez votre fichier d'icône personnalisé `omni_launcher.ico` dans le même dossier que le script.
2. Exécutez la commande de compilation suivante :
   ```bash
   pyinstaller --clean --noconsole --onedir --icon=omni_launcher.ico omni_launcher.py
   ```
3. **Installation sur votre PC :** - Une fois la compilation terminée, récupérez le **dossier complet** nommé `omni_launcher` qui a été généré dans le sous-dossier `dist/`.
   - Placez ce dossier à l'emplacement définitif de votre choix sur votre ordinateur (par exemple dans `C:\Program Files\` ou `D:\Logiciels\`).
   - Ouvrez ce dossier, faites un **clic droit** sur `omni_launcher.exe` > **Afficher d'autres options** > **Envoyer vers** > **Bureau (créer un raccourci)**.

*(Astuce : Pour automatiser vos futures mises à jour de code, vous pouvez ajouter l'argument `--distpath="D:\Votre\Chemin\Final"` à la commande de compilation pour écraser directement l'ancienne version de l'application).*

---

## 🛠️ Pour les développeurs (.gitignore)

Si vous clonez ce projet ou que vous souhaitez le versionner sur GitHub, il est fortement recommandé de créer un fichier `.gitignore` à la racine pour éviter d'envoyer les fichiers temporaires de compilation en ligne :

```text
# Environnements virtuels
venv/
.venv/
env/

# Fichiers de compilation PyInstaller
build/
dist/
*.spec

# Caches Python
__pycache__/
*.pyc

# Fichiers IDE (PyCharm, VSCode)
.idea/
.vscode/
```

---
