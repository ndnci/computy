# Validation locale

Validation du 29 octobre 2026, sur Linux. Aucun service distant ni tunnel public n’a été ouvert pour les essais.

| Vérification | Résultat |
|---|---|
| Moteur Rust | 43 tests réussis : domaine, firmware, vraies boucles RAM/disque/CPU, annulation, historique, préférences, MCP, import Windows, télémétrie réelle, classement des processus et cumul VRAM multi-GPU |
| Analyse Rust | `cargo fmt --check` et Clippy, toutes les cibles, `-D warnings` |
| Intégration Node | 17 tests réussis, dont CLI réelle, moniteur passif, paramètres de barre système, noms de marque dans les 25 langues et clients du SDK MCP officiel en stdio/HTTP |
| Interface Playwright | Sélections indépendantes, progression, historique/comparaison, deux relances réelles avec paramètres identiques et nouveaux identifiants, cases indépendantes, mesures/processus, spinner USB, notifications et guide Secure Boot ; captures clair/sombre |
| Internationalisation | 25 langues parcourues, directions LTR/RTL, thème système et choix explicite, 3 largeurs ; captures supplémentaires dans 5 langues et les deux thèmes |
| Bureau natif | Binaire Tauri, IPC Rust réel, trois indicateurs persistants, trois cycles RAM et deux cycles globaux, états Active/Passive relus sur D-Bus, activation et menu réels, icône d’application récupérée, mesures/processus réels, diagnostics RAM/disque/CPU, thème enregistré ; 8 captures WebKitGTK, 1 test du rendu des trois styles et 1 test D-Bus isolé |
| Compilation | Interface de production, paquet Linux `.deb`, CLI native et vérification Tauri dans Docker |
| Dépendances JavaScript | `npm audit` : aucun avis de vulnérabilité déclaré au moment de la vérification |

Les résultats automatisés des essais UI sont dans [`screenshots/capture-results.json`](screenshots/capture-results.json), [`locales-results.json`](screenshots/locales-results.json) et [`native-results.json`](screenshots/native-results.json) et [`history-monitor-results.json`](screenshots/history-monitor-results.json). Le nouveau scénario des styles est dans [`tray-style-results.json`](screenshots/tray-style-results.json). Ils sont générés par les scripts de validation, sans valeurs de santé simulées.

## Reproduire

```bash
npm ci
npm run build:engine
npm run test:native
npm test
npm run build
npm run test:ui
```

`test:ui` lance son propre serveur sur un port local disponible, crée un répertoire de données temporaire et le supprime après les captures. Il utilise Chrome installé dans `/usr/bin/google-chrome` ; `PLAYWRIGHT_CHROMIUM_EXECUTABLE` permet de choisir un autre Chromium de test. Ce navigateur appartient à l’outillage de développement, pas à la distribution Tauri.

Le contrôle natif Linux utilise [tauri-driver et WebKitWebDriver](https://v2.tauri.app/develop/tests/webdriver/manual-setup/), avec un affichage Xvfb. Une image dédiée est fournie :

```bash
docker build --target desktop-qa -t computy-desktop-qa .
# Préparer release/computy avec le binaire Linux compilé.
docker run --rm --network none --cap-drop ALL \
  --security-opt no-new-privileges:true --shm-size=256m \
  -e LIBGL_ALWAYS_SOFTWARE=1 \
  -e WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 \
  -v "$PWD:/workspace:ro" \
  -v "$PWD/docs/screenshots:/workspace/docs/screenshots:rw" \
  computy-desktop-qa dbus-run-session -- \
  xvfb-run -a -s '-screen 0 1600x1200x24' node scripts/check-desktop.mjs
```

L’exception de sandbox WebKit concerne uniquement ce conteneur de test sans réseau, sans capacités Linux et avec les sources en lecture seule. Elle n’est pas ajoutée au binaire ou aux commandes de lancement de l’application. Le compte `computy` de cette image a l’UID 1000 et doit pouvoir écrire dans le répertoire des captures monté.

## Portée

Windows et macOS disposent des adaptateurs et d’une matrice de compilation CI, mais leurs paquets, l’intégration au démarrage et le diagnostic Windows n’ont pas été exécutés physiquement dans cet environnement Linux. Les parseurs SMBIOS et Windows, dont les événements au démarrage, sont testés avec des données de référence.

Une compilation et des tests réussis ne certifient pas l’état de tous les composants. Les captures conservent les contrôles inconnus ou inaccessibles. Les limites de couverture, de firmware, de caches et de télémétrie sont exposées dans les rapports et dans la [méthodologie](DIAGNOSTICS.md).

Le parcours guidé et le composant avant démarrage de la version 0.1.1 sont détaillés dans [BOOT-DIAGNOSTICS.md](BOOT-DIAGNOSTICS.md). La sélection GRUB et la préparation USB réelles ne sont pas exécutées sur l’hôte pendant les vérifications.
