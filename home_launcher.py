# COMMANDE DE GENERATION DE .EXE : pyinstaller --clean --noconsole --onedir --icon=home_launcher.ico home_launcher.py

import io
import os
import shutil
import subprocess
import time
import tkinter as tk
from tkinter import filedialog, messagebox, ttk

# Importation pour les images
try:
    from PIL import Image, ImageDraw, ImageFile, ImageTk

    ImageFile.LOAD_TRUNCATED_IMAGES = True
except ImportError:
    _root = tk.Tk()
    _root.withdraw()
    messagebox.showerror("Erreur", "Installez Pillow : pip install Pillow")
    raise SystemExit(1)

# Importation pour l'extraction et la création automatique de raccourcis
try:
    import win32com.client
    from icoextract import IconExtractor
except ImportError:
    _root = tk.Tk()
    _root.withdraw()
    messagebox.showerror(
        "Erreur",
        "Pour l'extraction automatique, tapez dans l'invite de commande :\n\npip install pywin32 icoextract",
    )
    raise SystemExit(1)

# ---------------------------------------------------------------------------
# Constantes
# ---------------------------------------------------------------------------
DOSSIER_JEUX = r"D:\Games\RACCOURCIS_JEUX"
DOSSIER_IMAGES = os.path.join(DOSSIER_JEUX, "images")
RESOLUTION = "1200x800"

TAILLE_ICONE_NORMAL = 140
TAILLE_ICONE_MAXIMISE = 150
MAX_COLONNES_RESET = 25  # Nombre de colonnes au-delà desquelles on réinitialise le poids

EXTENSIONS_IMAGES = (".png", ".jpg", ".ico")
CARACTERES_INTERDITS = set('\\/:*?"<>|')

# Couleurs de l'interface
BG_MAIN = "#121212"
BG_BTN = "#2a2a2a"
BG_BTN_HOVER = "#3a3a3a"
BG_MENU = "#1a1a1a"
BG_ENTRY = "#2a2a2a"
COLOR_ACCENT = "#deff9a"
COLOR_FG = "white"
COLOR_FG_MUTED = "#888888"
COLOR_BORDER = "#4a4a4a"


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _image_valide(chemin: str) -> bool:
    """Renvoie True si le fichier existe et n'est pas vide."""
    return os.path.isfile(chemin) and os.path.getsize(chemin) > 0


def _taille_depuis_etat(etat_fenetre: str) -> int:
    """Renvoie la taille d'icône correspondant à l'état de la fenêtre."""
    return TAILLE_ICONE_MAXIMISE if etat_fenetre == "zoomed" else TAILLE_ICONE_NORMAL


def _creer_shell():
    """Instancie et renvoie un objet WScript.Shell."""
    return win32com.client.Dispatch("WScript.Shell")


# ---------------------------------------------------------------------------
# Classe principale
# ---------------------------------------------------------------------------

class GameLauncherApp:
    def __init__(self, root: tk.Tk) -> None:
        self.root = root
        self.root.title("Home Launcher")
        self.root.geometry(RESOLUTION)
        self.root.configure(bg=BG_MAIN)

        try:
            self.root.iconbitmap("home_launcher.ico")
        except tk.TclError:
            pass

        self.boutons_jeux: list[tk.Button] = []
        self.images_references: list[ImageTk.PhotoImage] = []
        self.raccourcis_liste: list[dict] = []
        self.colonnes_actuelles: int = 0
        self.est_maximise_precedent: bool = False

        self._construire_interface()

        os.makedirs(DOSSIER_IMAGES, exist_ok=True)

        self.analyser_dossier_jeux()
        self.redessiner_bibliotheque(TAILLE_ICONE_NORMAL)

    # ------------------------------------------------------------------
    # Construction de l'interface
    # ------------------------------------------------------------------

    def _construire_interface(self) -> None:
        """Crée tous les widgets statiques de la fenêtre principale."""
        # En-tête
        header_frame = tk.Frame(self.root, bg=BG_MAIN)
        header_frame.pack(fill="x", padx=30, pady=20)

        tk.Label(
            header_frame,
            text="Bibliothèque",
            font=("Segoe UI", 24, "bold"),
            fg=COLOR_FG,
            bg=BG_MAIN,
        ).pack(side="left")

        tk.Button(
            header_frame,
            text="+ Ajouter un jeu",
            font=("Segoe UI", 12, "bold"),
            fg=COLOR_FG,
            bg=BG_BTN,
            activebackground=BG_BTN_HOVER,
            activeforeground=COLOR_FG,
            bd=0,
            cursor="hand2",
            padx=15,
            pady=8,
            command=self.ajouter_nouveau_jeu,
        ).pack(side="right")

        # Zone scrollable
        self.canvas = tk.Canvas(self.root, bg=BG_MAIN, highlightthickness=0)
        self.scrollbar = ttk.Scrollbar(self.root, orient="vertical", command=self.canvas.yview)
        self.scrollable_frame = tk.Frame(self.canvas, bg=BG_MAIN)

        self.scrollable_frame.bind(
            "<Configure>",
            lambda e: self.canvas.configure(scrollregion=self.canvas.bbox("all")),
        )

        self.canvas_window = self.canvas.create_window(
            (0, 0), window=self.scrollable_frame, anchor="nw"
        )
        self.canvas.configure(yscrollcommand=self.scrollbar.set)

        self.canvas.pack(side="left", fill="both", expand=True, padx=20, pady=10)
        self.scrollbar.pack(side="right", fill="y")

        self.canvas.bind_all("<MouseWheel>", self._on_mousewheel)
        self.canvas.bind("<Configure>", self._on_canvas_resize)

    # ------------------------------------------------------------------
    # Gestion des jeux
    # ------------------------------------------------------------------

    def ajouter_nouveau_jeu(self) -> None:
        fichier_exe = filedialog.askopenfilename(
            title="Sélectionnez l'exécutable du jeu",
            filetypes=[("Fichiers exécutables", "*.exe")],
        )
        if not fichier_exe:
            return

        nom_jeu = os.path.splitext(os.path.basename(fichier_exe))[0]
        chemin_raccourci = os.path.join(DOSSIER_JEUX, f"{nom_jeu}.lnk")

        try:
            shell = _creer_shell()
            shortcut = shell.CreateShortCut(chemin_raccourci)
            shortcut.TargetPath = fichier_exe
            shortcut.WorkingDirectory = os.path.dirname(fichier_exe)
            shortcut.Save()

            time.sleep(0.3)
            self._rafraichir_bibliotheque()
        except Exception as e:
            messagebox.showerror("Erreur", f"Erreur lors de la création du raccourci :\n{e}")

    def analyser_dossier_jeux(self) -> None:
        """Scanne le dossier, nettoie les raccourcis invalides et peuple raccourcis_liste."""
        os.makedirs(DOSSIER_JEUX, exist_ok=True)

        shell = _creer_shell()

        raccourcis = [
            f for f in os.listdir(DOSSIER_JEUX) if f.lower().endswith(".lnk")
        ]

        for fichier in raccourcis:
            nom_jeu = os.path.splitext(fichier)[0]
            chemin_complet = os.path.join(DOSSIER_JEUX, fichier)

            # Vérification de la cible
            try:
                shortcut = shell.CreateShortCut(chemin_complet)
                fichier_cible = shortcut.TargetPath

                if not fichier_cible or not os.path.exists(fichier_cible):
                    print(f"Nettoyage : Le jeu '{nom_jeu}' est introuvable. Suppression du raccourci.")
                    self._supprimer_raccourci_et_images(chemin_complet, nom_jeu)
                    continue
            except Exception as e:
                print(f"Erreur lors de la vérification de '{nom_jeu}' : {e}")
                continue

            # Extraction de l'icône si nécessaire
            chemins_images = {
                ext: os.path.join(DOSSIER_IMAGES, f"{nom_jeu}{ext}")
                for ext in EXTENSIONS_IMAGES
            }
            if not any(_image_valide(p) for p in chemins_images.values()):
                self.extraire_icone_automatique(chemin_complet, chemins_images[".ico"])

            # Sélection de la meilleure image disponible
            source_img = next(
                (chemins_images[ext] for ext in EXTENSIONS_IMAGES if _image_valide(chemins_images[ext])),
                None,
            )

            self.raccourcis_liste.append({
                "nom": nom_jeu,
                "chemin": chemin_complet,
                "source_img": source_img,
            })

    def extraire_icone_automatique(self, chemin_lnk: str, chemin_sortie_ico: str) -> None:
        """Extrait l'icône d'un raccourci .lnk et la sauvegarde en .ico."""
        try:
            shell = _creer_shell()
            shortcut = shell.CreateShortCut(chemin_lnk)
            fichier_source = shortcut.TargetPath

            # Priorité à l'emplacement d'icône personnalisé
            icon_loc = shortcut.IconLocation
            if icon_loc and "," in icon_loc:
                source_potentielle = icon_loc.split(",")[0].strip().strip('"')
                if os.path.exists(source_potentielle):
                    fichier_source = source_potentielle

            if not fichier_source or not os.path.exists(fichier_source):
                return

            if fichier_source.lower().endswith(".ico"):
                shutil.copy(fichier_source, chemin_sortie_ico)
                return

            extractor = IconExtractor(fichier_source)
            extractor.export_icon(chemin_sortie_ico)

            # Supprimer le fichier vide si l'extraction a échoué silencieusement
            if os.path.exists(chemin_sortie_ico) and os.path.getsize(chemin_sortie_ico) == 0:
                os.remove(chemin_sortie_ico)

        except Exception as e:
            print(f"Erreur d'extraction d'icône pour '{chemin_lnk}' : {e}")
            if os.path.exists(chemin_sortie_ico):
                try:
                    os.remove(chemin_sortie_ico)
                except OSError:
                    pass

    # ------------------------------------------------------------------
    # Chargement des images
    # ------------------------------------------------------------------

    def creer_image_par_defaut(self, taille: int) -> Image.Image:
        """Crée une image de remplacement quand aucune icône n'est disponible."""
        img = Image.new("RGB", (taille, taille), color=BG_BTN)
        draw = ImageDraw.Draw(img)
        draw.rectangle([(0, 0), (taille - 1, taille - 1)], outline=COLOR_BORDER, width=2)
        return img

    def charger_image_securise(self, chemin_image: str, taille_icone: int) -> Image.Image:
        """Charge une image de façon robuste avec fallbacks pour les .ico corrompus."""
        try:
            img_pil = Image.open(chemin_image)

            if chemin_image.lower().endswith(".ico"):
                try:
                    n_frames = getattr(img_pil, "n_frames", 1)
                    if n_frames > 1:
                        # Sélectionner la frame avec la plus grande résolution
                        tailles = []
                        for i in range(n_frames):
                            img_pil.seek(i)
                            tailles.append((img_pil.size[0], i))
                        meilleure_frame = max(tailles, key=lambda x: x[0])[1]
                        img_pil.seek(meilleure_frame)
                    img_pil.load()
                except Exception as frame_err:
                    print(f"Calque ignoré pour '{chemin_image}' : {frame_err}")
            else:
                img_pil.load()

            return img_pil

        except Exception as e:
            print(f"Erreur Pillow pour '{chemin_image}' : {e}")

            # Tentative de sauvetage : chercher un PNG embarqué dans le .ico
            if chemin_image.lower().endswith(".ico"):
                try:
                    with open(chemin_image, "rb") as f:
                        data = f.read()
                    signature_png = b"\x89PNG\r\n\x1a\n"
                    index_png = data.find(signature_png)
                    if index_png != -1:
                        img_secours = Image.open(io.BytesIO(data[index_png:]))
                        img_secours.load()
                        return img_secours
                except Exception as secours_err:
                    print(f"  -> Échec du sauvetage PNG embarqué : {secours_err}")

            return self.creer_image_par_defaut(taille_icone)

    # ------------------------------------------------------------------
    # Affichage de la bibliothèque
    # ------------------------------------------------------------------

    def redessiner_bibliotheque(self, taille_icone: int) -> None:
        """Détruit et recrée tous les boutons de jeux."""
        for btn in self.boutons_jeux:
            btn.destroy()
        self.boutons_jeux.clear()
        self.images_references.clear()

        # Supprimer les éventuels labels résiduels (ex: message "aucun jeu")
        for widget in self.scrollable_frame.winfo_children():
            if isinstance(widget, tk.Label):
                widget.destroy()

        if not self.raccourcis_liste:
            tk.Label(
                self.scrollable_frame,
                text="Aucun jeu dans votre bibliothèque.\nCliquez sur '+ Ajouter un jeu' en haut à droite.",
                font=("Segoe UI", 12),
                fg=COLOR_FG_MUTED,
                bg=BG_MAIN,
                pady=50,
            ).pack()
            return

        for jeu in self.raccourcis_liste:
            if jeu["source_img"]:
                img_pil = self.charger_image_securise(jeu["source_img"], taille_icone)
            else:
                img_pil = self.creer_image_par_defaut(taille_icone)

            img_pil = img_pil.convert("RGBA").resize(
                (taille_icone, taille_icone), Image.Resampling.LANCZOS
            )
            photo = ImageTk.PhotoImage(img_pil)
            self.images_references.append(photo)

            btn = tk.Button(
                self.scrollable_frame,
                text=jeu["nom"],
                image=photo,
                compound="top",
                font=("Segoe UI", 10, "bold"),
                fg=COLOR_FG,
                bg=BG_MAIN,
                activebackground="#1e1e1e",
                activeforeground=COLOR_FG,
                bd=0,
                cursor="hand2",
                pady=10,
                wraplength=taille_icone + 20,
            )

            btn.bind("<Double-Button-1>", lambda e, ch=jeu["chemin"]: self.lancer_jeu(ch))
            btn.bind("<Button-3>", lambda e, j=jeu, b=btn: self.afficher_menu_contextuel(e, j, b))

            self.boutons_jeux.append(btn)

        self.colonnes_actuelles = 0
        self._on_canvas_resize(None)

    def afficher_menu_contextuel(self, event: tk.Event, jeu: dict, btn: tk.Button) -> None:
        menu = tk.Menu(
            self.root,
            tearoff=0,
            bg=BG_MENU,
            fg=COLOR_FG,
            activebackground=BG_BTN_HOVER,
            activeforeground=COLOR_FG,
            bd=1,
        )
        menu.add_command(
            label="Ouvrir le dossier du jeu",
            command=lambda: self.ouvrir_dossier_jeu(jeu["chemin"]),
        )
        menu.add_command(
            label="Renommer le jeu",
            command=lambda: self.activer_renommage_inline(jeu, btn),
        )
        menu.add_command(
            label="Retirer du launcher",
            command=lambda: self.retirer_jeu_launcher(jeu),
        )
        menu.add_separator()
        menu.add_command(
            label="Supprimer définitivement le jeu",
            command=lambda: self.supprimer_jeu(jeu),
        )
        menu.post(event.x_root, event.y_root)

    # ------------------------------------------------------------------
    # Actions sur les jeux
    # ------------------------------------------------------------------

    def lancer_jeu(self, chemin_raccourci: str) -> None:
        try:
            os.startfile(chemin_raccourci)
            self.root.after(3000, self.root.destroy)
        except OSError as e:
            if getattr(e, "winerror", None) == 1223:
                # Erreur UAC : fallback via l'Explorateur Windows en processus détaché
                try:
                    subprocess.Popen(
                        ["explorer.exe", chemin_raccourci],
                        close_fds=True,
                        creationflags=0x00000008,  # DETACHED_PROCESS
                    )
                    self.root.after(3000, self.root.destroy)
                except Exception as fallback_err:
                    messagebox.showerror(
                        "Erreur", f"Même l'Explorateur n'a pas pu lancer le jeu :\n{fallback_err}"
                    )
            else:
                messagebox.showerror("Erreur", f"Erreur système lors du lancement :\n{e}")
        except Exception as e:
            messagebox.showerror("Erreur", f"Impossible de lancer le jeu :\n{e}")

    def ouvrir_dossier_jeu(self, chemin_lnk: str) -> None:
        try:
            shell = _creer_shell()
            shortcut = shell.CreateShortCut(chemin_lnk)
            fichier_cible = shortcut.TargetPath

            if fichier_cible and os.path.exists(fichier_cible):
                os.startfile(os.path.dirname(fichier_cible))
            else:
                print(f"Impossible de localiser le fichier cible pour '{chemin_lnk}'")
        except Exception as e:
            print(f"Erreur lors de l'ouverture du dossier : {e}")

    def retirer_jeu_launcher(self, jeu: dict) -> None:
        """Supprime le raccourci .lnk et ses images sans toucher au jeu installé."""
        reponse = messagebox.askyesno(
            "Retirer du launcher",
            f"Voulez-vous vraiment retirer '{jeu['nom']}' de votre bibliothèque ?\n\n"
            "(Rassurez-vous, le jeu restera installé sur votre ordinateur, seul le raccourci sera supprimé.)",
        )
        if not reponse:
            return

        try:
            self._supprimer_raccourci_et_images(jeu["chemin"], jeu["nom"])
            self._rafraichir_bibliotheque()
        except Exception as e:
            messagebox.showerror("Erreur", f"Impossible de retirer le jeu du launcher :\n{e}")

    def supprimer_jeu(self, jeu: dict) -> None:
        """Supprime définitivement le dossier d'installation du jeu."""
        reponse = messagebox.askyesno(
            "Confirmation de suppression",
            f"Êtes-vous sûr de vouloir supprimer COMPLÈTEMENT le jeu '{jeu['nom']}' ?\n\n"
            "Cette action effacera définitivement tout le dossier d'installation du jeu sur votre disque dur.",
        )
        if not reponse:
            return

        try:
            shell = _creer_shell()
            shortcut = shell.CreateShortCut(jeu["chemin"])
            fichier_cible = shortcut.TargetPath

            if fichier_cible and os.path.exists(fichier_cible):
                dossier_parent = os.path.dirname(fichier_cible)
                # Sécurité : ne jamais supprimer un chemin trop court (ex: C:\)
                if len(dossier_parent) > 4 and os.path.exists(dossier_parent):
                    shutil.rmtree(dossier_parent)
                    print(f"Dossier supprimé : {dossier_parent}")

            self._supprimer_raccourci_et_images(jeu["chemin"], jeu["nom"])
            self._rafraichir_bibliotheque()
            messagebox.showinfo("Succès", f"Le jeu '{jeu['nom']}' a été supprimé avec succès.")

        except Exception as e:
            messagebox.showerror("Erreur", f"Impossible de supprimer complètement le jeu :\n{e}")

    # ------------------------------------------------------------------
    # Renommage inline
    # ------------------------------------------------------------------

    def activer_renommage_inline(self, jeu: dict, btn: tk.Button) -> None:
        entry = tk.Entry(
            btn,
            font=("Segoe UI", 10, "bold"),
            justify="center",
            bg=BG_ENTRY,
            fg=COLOR_FG,
            insertbackground=COLOR_FG,
            bd=0,
            highlightthickness=1,
            highlightcolor=COLOR_ACCENT,
        )
        entry.insert(0, jeu["nom"])
        entry.place(relx=0.5, rely=0.88, anchor="center", relwidth=0.9, height=25)
        entry.focus_set()
        entry.select_range(0, "end")

        def valider_renommage(event=None):
            nouveau_nom = entry.get().strip()
            entry.destroy()
            self.executer_renommage(jeu, nouveau_nom)

        def annuler_renommage(event=None):
            entry.destroy()

        entry.bind("<Return>", valider_renommage)
        entry.bind("<Escape>", annuler_renommage)
        entry.bind("<FocusOut>", annuler_renommage)

    def executer_renommage(self, jeu: dict, nouveau_nom: str) -> None:
        if not nouveau_nom or nouveau_nom == jeu["nom"]:
            return

        if any(c in CARACTERES_INTERDITS for c in nouveau_nom):
            messagebox.showerror(
                "Erreur", 'Le nom contient des caractères interdits (\\ / : * ? " < > |)'
            )
            return

        nouveau_chemin_lnk = os.path.join(DOSSIER_JEUX, f"{nouveau_nom}.lnk")
        if os.path.exists(nouveau_chemin_lnk):
            messagebox.showerror("Erreur", "Un jeu avec ce nom existe déjà dans votre bibliothèque.")
            return

        try:
            os.rename(jeu["chemin"], nouveau_chemin_lnk)

            for ext in EXTENSIONS_IMAGES:
                ancien = os.path.join(DOSSIER_IMAGES, f"{jeu['nom']}{ext}")
                nouveau = os.path.join(DOSSIER_IMAGES, f"{nouveau_nom}{ext}")
                if os.path.exists(ancien):
                    try:
                        os.rename(ancien, nouveau)
                    except OSError as img_err:
                        print(f"Impossible de renommer l'image '{ext}' : {img_err}")

            self._rafraichir_bibliotheque()

        except Exception as e:
            messagebox.showerror("Erreur", f"Impossible de renommer le jeu :\n{e}")

    # ------------------------------------------------------------------
    # Gestion du redimensionnement
    # ------------------------------------------------------------------

    def _on_canvas_resize(self, event) -> None:
        largeur_canvas = event.width if event else self.canvas.winfo_width()
        if largeur_canvas <= 1:
            return

        self.canvas.itemconfig(self.canvas_window, width=largeur_canvas)
        est_maximise = self.root.state() == "zoomed"

        if est_maximise != self.est_maximise_precedent and event is not None:
            self.est_maximise_precedent = est_maximise
            nouvelle_taille = _taille_depuis_etat(self.root.state())
            self.root.after(10, lambda: self.redessiner_bibliotheque(nouvelle_taille))
            return

        taille_icone = _taille_depuis_etat(self.root.state())
        colonnes = max(1, largeur_canvas // (taille_icone + 40))

        if colonnes != self.colonnes_actuelles and self.boutons_jeux:
            self.colonnes_actuelles = colonnes

            for index, btn in enumerate(self.boutons_jeux):
                btn.grid(
                    row=index // colonnes,
                    column=index % colonnes,
                    padx=15,
                    pady=15,
                    sticky="n",
                )

            # Réinitialiser le poids des colonnes
            for i in range(MAX_COLONNES_RESET):
                self.scrollable_frame.grid_columnconfigure(i, weight=0)
            for i in range(colonnes):
                self.scrollable_frame.grid_columnconfigure(i, weight=1)

    def _on_mousewheel(self, event: tk.Event) -> None:
        self.canvas.yview_scroll(int(-1 * (event.delta / 120)), "units")

    # ------------------------------------------------------------------
    # Utilitaires internes
    # ------------------------------------------------------------------

    def _supprimer_raccourci_et_images(self, chemin_lnk: str, nom_jeu: str) -> None:
        """Supprime le fichier .lnk et toutes les images associées au jeu."""
        if os.path.exists(chemin_lnk):
            os.remove(chemin_lnk)
        for ext in EXTENSIONS_IMAGES:
            chemin_img = os.path.join(DOSSIER_IMAGES, f"{nom_jeu}{ext}")
            if os.path.exists(chemin_img):
                try:
                    os.remove(chemin_img)
                except OSError as e:
                    print(f"Impossible de supprimer l'image '{chemin_img}' : {e}")

    def _rafraichir_bibliotheque(self) -> None:
        """Recharge les données et redessine la bibliothèque."""
        self.raccourcis_liste.clear()
        self.analyser_dossier_jeux()
        taille = _taille_depuis_etat(self.root.state())
        self.redessiner_bibliotheque(taille)


# ---------------------------------------------------------------------------
# Point d'entrée
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    root = tk.Tk()
    app = GameLauncherApp(root)
    root.mainloop()