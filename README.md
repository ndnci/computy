<p align="center"><img src="public/computy-logo.png" width="120" alt="Logo Computy" /></p>

# Computy

**Votre matériel, en clair.** Une application de bureau **Rust + Tauri**, une **CLI native** et un **serveur MCP** pour comprendre, vérifier et suivre son ordinateur.

Une création **[NDNCI](https://www.ndnci.com)**. Aucun compte, aucune télémétrie. L’historique reste sur votre machine.

![Computy : présentation en clair et sombre](docs/screenshots/presentation-overview-with-history.png)

## Ce que Computy permet

- **Choisir ses contrôles avec des cases à cocher** : RAM seule, stockage seul, RAM + stockage, processeur ou bilan complet.
- Lire l’inventaire réel : mémoire visible, modules et capacité maximale déclarés par le firmware, débits configurés, format et remplacement à confirmer, processeur, volumes et capteurs exposés.
- Vérifier la RAM avec plusieurs motifs et inversions, analyser les données **SMART/NVMe** accessibles, relire un fichier temporaire synchronisé et vérifier des calculs CPU.
- Suivre la progression réelle, interrompre un diagnostic et conserver ses résultats partiels.
- Conserver un instantané du matériel pour chaque bilan, le renommer, l’exporter en JSON et comparer **2 à 4 bilans**.
- **Relancer un bilan** depuis sa ligne ou son rapport : les mêmes paramètres produisent un nouveau diagnostic, et l’original reste conservé. Toute la ligne ouvre le rapport ; les cases et actions restent indépendantes. Un rapport Windows avant démarrage rouvre l’assistant et demande une nouvelle confirmation.
- Suivre RAM/CPU/GPU dans des **indicateurs de la barre système**, activés par défaut et configurables, avec des volumes au choix. Un clic montre les **10 premiers processus**, et **Voir tout** ouvre la liste complète avec recherche et tri. Les valeurs absentes restent explicites. [Fonctionnement et limites du moniteur](docs/MONITORING.md).
- Utiliser le thème **système par défaut**, ou choisir **clair/sombre** ; choisir parmi **25 langues**, avec formats locaux et sens de lecture adapté.
- Retrouver les mêmes contrôles, rapports, préférences et comparaisons depuis le terminal ou un agent MCP.

**Un résultat sans erreur signifie qu’aucune erreur n’a été observée dans les conditions du test. Il ne certifie pas l’absence de panne.** Computy affiche les données absentes et la portée de chaque contrôle. La note sur 10 résume les contrôles exploitables ; elle n’est pas une probabilité de bonne santé.

## Démarrer

### Tester immédiatement sur le PC Linux de développement

Les binaires ont déjà été compilés dans `release/`. Depuis la racine du projet :

```bash
# Ouvrir l’application native, sans compilation ni Docker
./release/computy

# Ou lancer un diagnostic léger RAM + stockage dans le terminal
./release/computy-cli diagnose --profile quick --components memory,storage
```

Dans l’application, ouvrir **Diagnostics**, cocher les composants souhaités, choisir **Premier regard**, puis **Lancer le diagnostic**. L’historique conserve automatiquement le rapport.

Sur Ubuntu/Debian, l’installation facultative du paquet ajoute aussi Computy au menu des applications :

```bash
sudo apt install ./release/Computy_0.1.3_amd64.deb
```

Les bibliothèques d’exécution sont déjà présentes sur le PC de développement vérifié. Les binaires de `release/` sont des artefacts locaux exclus de Git ; après un nouveau clonage, les reconstruire avec les commandes ci-dessous.

### Application de bureau

Prérequis : Node.js 22+, Rust stable (1.95+), et les [dépendances système Tauri](https://v2.tauri.app/start/prerequisites/). Linux utilise WebKitGTK, macOS WKWebView et Windows WebView2. **Computy n’embarque pas Chromium.**

```bash
npm ci
npm run desktop
```

Créer le paquet natif de la plateforme courante :

```bash
npm run package
```

Les paquets sont produits dans `src-tauri/target/release/bundle/`. Windows et macOS se compilent sur leurs systèmes respectifs ; le workflow pour les trois plateformes est conservé dans `.github/workflows/checks.yml.disabled` et les actions automatiques sont désactivées pour le moment. Les versions ne sont pas signées ni notariées à ce stade.

Le paquet Linux validé localement se trouve dans [`release/Computy_0.1.3_amd64.deb`](release/Computy_0.1.3_amd64.deb), avec le binaire de bureau et la CLI dans `release/`. Ce répertoire de compilation est exclu de Git. Taille du paquet : environ **7,6 Mio**.

### Développer et vérifier avec Docker

Docker fournit Rust et les dépendances Linux de Tauri, sans installation du compilateur sur l’hôte.

```bash
# Vérifications du moteur, de l’interface et de l’intégration Tauri
docker compose run --build --rm verify

# Aperçu de développement sur http://127.0.0.1:1420
docker compose up --build dev

# Paquet Linux .deb, copié dans ./release
docker compose run --build --rm desktop-build
```

**Un conteneur expose son environnement d’exécution.** L’aperçu Docker sert au développement ; il ne doit pas être interprété comme un inventaire complet de l’hôte. Pour diagnostiquer son PC, exécuter le binaire natif sur ce PC. Aucune option `--privileged` n’est demandée.

### CLI sans interface

```bash
cargo build --release --manifest-path native/Cargo.toml
./native/target/release/computy-cli --help

# Inventaire complet en JSON
./native/target/release/computy-cli --json inventory

# RAM seule
./native/target/release/computy-cli diagnose --components memory --memory-mib 256 --passes 2

# RAM + stockage
./native/target/release/computy-cli diagnose --components memory,storage --label "Avant remplacement"

# Bilan complet, profil léger
./native/target/release/computy-cli --json diagnose --profile quick --label "Après remplacement"

./native/target/release/computy-cli history
./native/target/release/computy-cli --json compare ID_AVANT ID_APRES
./native/target/release/computy-cli export ID --output rapport.json
./native/target/release/computy-cli rename ID "RAM après remplacement"
./native/target/release/computy-cli settings set --theme system --language fr

# Ouverture de l’application installée à la connexion (facultatif)
./native/target/release/computy-cli startup enable --desktop-path /chemin/absolu/Computy
```

`Ctrl+C` interrompt proprement un diagnostic. La progression va sur `stderr`, les résultats JSON sur `stdout`. Codes de sortie : `0` terminé sans anomalie bloquante, `1` erreur de commande, `2` anomalie détectée, `3` aucun contrôle exploitable, `130` interrompu. Un code `0` peut accompagner une couverture partielle : examiner `summary.status`, `checked/total` et chaque contrôle.

`--data-dir CHEMIN` permet d’isoler un historique. En version installée, la CLI et Tauri utilisent le répertoire de données de `com.ndnci.computy` pour l’utilisateur courant. Le mode développement utilise `.computy` ; passer `--data-dir .computy` à la CLI pour partager cet historique. `COMPUTY_DATA_DIR` configure également le répertoire du moteur en développement.

## Agents IA et MCP

Les transports utilisent le même moteur Rust. **Lecture seule par défaut** ; pas de shell arbitraire, d’écriture sur périphérique brut ou de commande de redémarrage exposés aux agents.

### En local : stdio

```json
{
  "mcpServers": {
    "computy": {
      "command": "/chemin/absolu/computy-cli",
      "args": ["mcp"]
    }
  }
}
```

Ajouter `--allow-diagnostics` pour autoriser le lancement et l’arrêt de tests bornés. Ajouter séparément `--allow-history-write` pour autoriser renommage, suppression, préférences et import d’un résultat Windows. Les outils indisponibles ne sont pas annoncés et les appels directs sont refusés.

### Accès temporaire via HTTP et tunnel facultatif

```bash
computy-cli mcp --http --port 8765 --ttl 900 --token-file /chemin/prive/computy.token

# Si cloudflared est déjà installé : publication explicitement demandée
computy-cli mcp --http --ttl 900 --cloudflare-tunnel
```

Le serveur écoute exclusivement sur **127.0.0.1**, génère un jeton de 256 bits, l’écrit dans un fichier privé (`0600` sur Unix), vérifie chaque en-tête `Authorization: Bearer …` et expire après la durée choisie (60 à 3600 secondes). Le jeton n’est jamais placé dans l’URL. Les origines de navigateur sont refusées par défaut ; seules des origines HTTPS exactes peuvent être autorisées avec `--allow-origin`.

Le tunnel transmet uniquement le port MCP et fixe l’en-tête Host local attendu. Il s’arrête avec le serveur lors d’un arrêt normal ou à l’expiration. **Aucun tunnel n’est ouvert automatiquement au démarrage de l’application.** Le client distant utilise l’URL HTTPS affichée par `cloudflared`, suivie de `/mcp`, et le jeton dans ses en-têtes.

Ce mode vise les clients MCP capables de fournir un jeton Bearer explicite. Il ne fournit pas un serveur OAuth ni une intégration universelle aux clients exigeant OAuth. Les [Quick Tunnels Cloudflare](https://developers.cloudflare.com/tunnel/get-started/quick-tunnels/) sont destinés au développement et aux accès temporaires ; un déploiement durable doit utiliser une configuration gérée avec contrôle d’accès.

Voir [la sécurité MCP](docs/SECURITY.md) pour les limites, les permissions et la révocation.

## Test de mémoire au démarrage

Le mode **Avant démarrage** est séparé des profils de session. L’assistant distingue **sans clé USB** et **avec clé USB**, vérifie la compatibilité et demande les accords avant préparation, écriture sur une clé et redémarrage. Le moteur intégré et ses sources GPL sont livrés avec Computy.

Sous Linux x86-64 compatible, Computy peut préparer une sélection GRUB pour un seul démarrage, ou copier le moteur sur une clé FAT32 sans formatage ni écrasement d’un chargeur existant. Secure Boot actif bloque le binaire non approuvé ; Computy ne désactive pas la protection. Les résultats Linux autonomes ne sont pas importés automatiquement et ne reçoivent aucun score supposé.

Windows utilise le diagnostic mémoire système avec import d’un événement récent au retour. La réouverture à la connexion est proposée activée dans ce parcours. Le parcours Apple reste dépendant du modèle.

Voir [le fonctionnement, les limites et les confirmations](docs/BOOT-DIAGNOSTICS.md).

### Mettre à jour une installation Ubuntu/Debian

Quitter Computy depuis le menu d’un indicateur (**Quitter Computy**), puis depuis la racine du projet :

```bash
sudo apt install ./release/Computy_0.1.3_amd64.deb
/usr/bin/computy
```

Le nom de paquet reste `computy` : APT remplace la version précédente par la 0.1.3. L’historique et les préférences restent dans le même répertoire utilisateur. Le bouton du header **Actualiser les informations** relit le matériel et confirme le résultat par une notification ; il ne met pas à jour l’application. Une copie portable lancée avec `./release/computy` est distincte de l’installation APT : utiliser le menu des applications ou `/usr/bin/computy` pour tester le paquet installé.

## Aperçus de l’application

### Moniteur et processus en direct

Trois styles au choix : **icône + pourcentage** par défaut, **anneau**, ou **icône qui se remplit**. Bleu sous 65 %, jaune de 65 à 89 %, rouge à partir de 90 %. Les indicateurs restent enregistrés pendant leur désactivation pour permettre leur réactivation. Un panneau détaillé affiche le composant, les premiers processus avec leurs icônes disponibles et les actions. Sur les versions GNOME qui imposent le menu natif au premier clic, sa première entrée ou un double clic ouvre le panneau.

| Clair | Sombre |
|---|---|
| ![Styles des indicateurs en clair](docs/screenshots/tray-styles-light.png) | ![Styles des indicateurs en sombre](docs/screenshots/tray-styles-dark.png) |
| ![Panneau RAM en clair](docs/screenshots/tray-panel-memory-light.png) | ![Panneau RAM en sombre](docs/screenshots/tray-panel-memory-dark.png) |

![Panneau Tauri avec icône réelle de Computy](docs/screenshots/native-linux-tray-panel.png)

| Clair | Sombre |
|---|---|
| ![Préférences du moniteur en clair](docs/screenshots/monitor-settings-light.png) | ![Préférences du moniteur en sombre](docs/screenshots/monitor-settings-dark.png) |
| ![Processus en direct en clair](docs/screenshots/processes-live-light.png) | ![Processus en direct en sombre](docs/screenshots/processes-live-dark.png) |
| ![Relancer depuis l’historique en clair](docs/screenshots/history-rerun-actions-light.png) | ![Relancer depuis l’historique en sombre](docs/screenshots/history-rerun-actions-dark.png) |

Les anneaux natifs restent dans la barre système lorsque la fenêtre est masquée. Le placement dépend du bureau ; les captures ci-dessus présentent les réglages et les mesures dans Computy.

Les captures montrent les données exposées par la machine de validation. Les bilans affichés sont issus de contrôles réellement exécutés, sans simulation de remplacement matériel.

### Tableau de bord

| Clair | Sombre |
|---|---|
| ![Tableau de bord clair](docs/screenshots/overview-with-history-light.png) | ![Tableau de bord sombre](docs/screenshots/overview-with-history-dark.png) |

### Choisir son diagnostic

![Diagnostic : présentation diagonale](docs/screenshots/presentation-diagnostics.png)

| Clair | Sombre |
|---|---|
| ![Diagnostic complet clair](docs/screenshots/diagnostics-light.png) | ![Diagnostic complet sombre](docs/screenshots/diagnostics-dark.png) |
| ![RAM seule clair](docs/screenshots/diagnostics-memory-only-light.png) | ![RAM seule sombre](docs/screenshots/diagnostics-memory-only-dark.png) |
| ![RAM et stockage clair](docs/screenshots/diagnostics-memory-storage-light.png) | ![RAM et stockage sombre](docs/screenshots/diagnostics-memory-storage-dark.png) |

### Progression et résultats

| Clair | Sombre |
|---|---|
| ![Progression claire](docs/screenshots/diagnostic-running-light.png) | ![Progression sombre](docs/screenshots/diagnostic-running-dark.png) |
| ![Résultats clairs](docs/screenshots/diagnostic-result-light.png) | ![Résultats sombres](docs/screenshots/diagnostic-result-dark.png) |

### Historique et comparaison

![Comparaison : présentation diagonale](docs/screenshots/presentation-comparison.png)

| Clair | Sombre |
|---|---|
| ![Historique clair](docs/screenshots/history-light.png) | ![Historique sombre](docs/screenshots/history-dark.png) |
| ![Comparaison claire](docs/screenshots/comparison-light.png) | ![Comparaison sombre](docs/screenshots/comparison-dark.png) |

### Matériel et préférences

| Clair | Sombre |
|---|---|
| ![Mémoire claire](docs/screenshots/memory-light.png) | ![Mémoire sombre](docs/screenshots/memory-dark.png) |
| ![Stockage clair](docs/screenshots/storage-light.png) | ![Stockage sombre](docs/screenshots/storage-dark.png) |
| ![Processeur clair](docs/screenshots/cpu-light.png) | ![Processeur sombre](docs/screenshots/cpu-dark.png) |
| ![Préférences claires](docs/screenshots/settings-light.png) | ![Préférences sombres](docs/screenshots/settings-dark.png) |
| ![À propos clair](docs/screenshots/about-light.png) | ![À propos sombre](docs/screenshots/about-dark.png) |

<details>
<summary>Fenêtre native Tauri Linux : clair et sombre</summary>

Ces captures supplémentaires viennent du binaire Tauri final exécuté avec WebKitGTK dans un conteneur de validation, sans serveur web de développement. Les contrôles RAM, fichier temporaire et CPU ont réellement été exécutés et leurs mesures relues dans les rapports.

| Clair | Sombre |
|---|---|
| ![Tauri Linux accueil clair](docs/screenshots/native-linux-overview-light.png) | ![Tauri Linux accueil sombre](docs/screenshots/native-linux-overview-dark.png) |
| ![Tauri Linux diagnostics clairs](docs/screenshots/native-linux-diagnostics-light.png) | ![Tauri Linux diagnostics sombres](docs/screenshots/native-linux-diagnostics-dark.png) |
| ![Tauri Linux résultat clair](docs/screenshots/native-linux-result-light.png) | ![Tauri Linux résultat sombre](docs/screenshots/native-linux-result-dark.png) |

</details>

<details>
<summary>Quelques langues : anglais, arabe, japonais, chinois et hindi</summary>

| Clair | Sombre |
|---|---|
| ![English light](docs/screenshots/overview-en-light.png) | ![English dark](docs/screenshots/overview-en-dark.png) |
| ![Arabe clair, de droite à gauche](docs/screenshots/overview-ar-light.png) | ![Arabe sombre, de droite à gauche](docs/screenshots/overview-ar-dark.png) |
| ![Japonais clair](docs/screenshots/overview-ja-light.png) | ![Japonais sombre](docs/screenshots/overview-ja-dark.png) |
| ![Chinois clair](docs/screenshots/overview-zh-CN-light.png) | ![Chinois sombre](docs/screenshots/overview-zh-CN-dark.png) |
| ![Hindi clair](docs/screenshots/overview-hi-light.png) | ![Hindi sombre](docs/screenshots/overview-hi-dark.png) |

</details>

### Résultat RAM expliqué

Le même rapport peut avoir des tests réussis et des compteurs indisponibles. L’interface les distingue ; la mémoire réellement testée et le nombre de passes sont visibles.

| Clair | Sombre |
|---|---|
| ![Résultat RAM expliqué, clair](docs/screenshots/ram-report-explained-light.png) | ![Résultat RAM expliqué, sombre](docs/screenshots/ram-report-explained-dark.png) |

### Parcours guidé avant démarrage

| Clair | Sombre |
|---|---|
| ![Sans clé USB, clair](docs/screenshots/boot-guide-internal-light.png) | ![Sans clé USB, sombre](docs/screenshots/boot-guide-internal-dark.png) |
| ![Avec clé USB, clair](docs/screenshots/boot-guide-usb-light.png) | ![Avec clé USB, sombre](docs/screenshots/boot-guide-usb-dark.png) |

Le composant technique avant démarrage en VM UEFI (son environnement graphique dépend du firmware) :

![Phase de test mémoire intégrée, QEMU UEFI](docs/screenshots/boot-engine-uefi.png)

## Architecture et vérification

```text
native/             Domaine, diagnostics, inventaire OS, historique, CLI, MCP
src-tauri/          Hôte de bureau Rust : commandes validées, fenêtres et intégration OS
src/                Interface React/TypeScript et catalogues i18n embarqués
scripts/            Aperçu natif, génération locale des langues, captures et présentations
tests/              Contrats de présentation et intégration CLI/MCP avec le SDK officiel
docs/               Méthodologie, sécurité, journal RGR et captures
Dockerfile          Compilation, vérifications, aperçu de développement
```

Le domaine et les diagnostics ne dépendent pas de l’interface. Les adaptateurs d’inventaire et d’historique passent par des interfaces dédiées. La CLI, Tauri et MCP utilisent le même service applicatif. Les rapports sont écrits atomiquement, avec une empreinte SHA-256 pour détecter les altérations accidentelles ; cette empreinte n’est pas une signature contre un attaquant possédant les droits de l’utilisateur.

```bash
npm run build:engine
npm test
npm run test:native
npm run build
npm run check:desktop

# Parcours complets, données isolées et captures automatiques
npm run test:ui
```

Les cycles **Red → Green → Refactor** sont décrits dans [le journal RGR](docs/RGR.md). La [méthodologie](docs/DIAGNOSTICS.md) explique les algorithmes, la notation et les sources primaires. La [documentation des langues](docs/I18N.md) décrit la génération hors ligne et les corrections éditoriales.

Le [compte rendu de validation](docs/VALIDATION.md) précise les résultats obtenus, le contrôle de la fenêtre native et les plateformes restant à valider sur une machine physique.
