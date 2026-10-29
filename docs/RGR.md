# Journal Red → Green → Refactor

Les tests ont été écrits et exécutés en échec avant l’implémentation des comportements associés. Les données artificielles servent aux tests de parseurs et à l’injection de corruption ; les captures de l’application utilisent les contrôles réels du moteur.

| Cycle | Red observé | Green / comportement | Refactor |
|---|---|---|---|
| Contrats du domaine et de l’inventaire | 12 tests en échec sur APIs non implémentées | Inconnu sans note ; minimum bloquant ; corruption injectée détectée ; SMART traité comme masque ; SMBIOS sans fréquence maximale inventée ; identifiants sûrs | Domaine séparé des sondes et de la persistance |
| Exécution réelle | 4 nouveaux tests en échec pour disque et CPU | Fichier synchronisé puis vérifié, nettoyage préservant le fichier témoin, annulation sans succès ; RAM réellement parcourue ; historique altéré rejeté | Fonctions de vérification et RAII du fichier/mémoire |
| Résultats au démarrage | 2 tests en échec sur import Windows | Fournisseur et ancienneté vérifiés ; événements d’erreur et interruption préservés | Import isolé du moteur de test en direct |
| Présentation et comparaison | Module de présentation absent, tests rouges | Données manquantes explicites, anomalie prioritaire, avertissements de protocole/machine | Fonctions de présentation indépendantes de React |
| MCP | 2 tests rouges avant gestion du protocole et de l’authentification | Permissions minimales, refus des mutations, jeton requis, expiration, origine et Host | Transport séparé du service applicatif |
| Préférences système | Test rouge : thème par défaut `light`, langue absente | Thème `system`, langue persistante, valeurs validées | Préférences communes CLI/Tauri/interface |
| Preuve SMART partielle | Test rouge : alerte matérielle masquée par erreur de transport | Une alerte explicite reste un échec, même avec télémétrie partielle | Priorité de l’observation matérielle sur disponibilité du transport |
| Parcours UI | Playwright ne pouvait pas cliquer la case native ; ancien résultat visible au début d’un nouveau test | Surface cliquable corrigée, ancien état retiré au lancement, deux vrais diagnostics puis comparaison | État applicatif de lancement clarifié |
| Parité des réglages et ECC partiel | Deux nouveaux tests rouges : réglage RAM ignoré par le transport ; erreur ECC non corrigée masquée par un compteur absent | Préférences communes avec surcharge explicite ; une erreur observée reste bloquante | Fusion des options dans l’adaptateur commun |
| En-tête Origin invalide | Test HTTP rouge : un Origin non ASCII atteignait la négociation de contenu | Refus explicite `403`, comme pour les origines interdites | Validation du transport avant traitement du protocole |
| Propriété du fichier temporaire | Contrat de création atomique absent | Une collision de nom refuse le test et préserve le fichier existant | Le garde RAII n’acquiert le fichier qu’après sa création effective |

## Vérifications

```bash
cargo test --manifest-path native/Cargo.toml
cargo clippy --manifest-path native/Cargo.toml --all-targets -- -D warnings
npm run build:engine
npm test
npm run build
npm run test:ui
cargo check --manifest-path src-tauri/Cargo.toml
```

Les tests d’intégration Node lancent réellement la CLI, vérifient les mesures, exportent et comparent les rapports et se connectent au MCP avec le SDK officiel en stdio et HTTP authentifié. Le test HTTP reste sur l’adresse locale et n’ouvre aucun tunnel.

`scripts/capture-ui.mjs` vérifie la sélection indépendante des trois composants, les modes RAM seule / RAM + stockage / complet, la progression, les rapports persistés et la comparaison. Chaque écran est capturé dans les deux thèmes. Les données de validation doivent être isolées avec `COMPUTY_DATA_DIR`.

Les catalogues de langues font l’objet de vérifications de couverture, de préservation des variables, de sens de lecture et de libellés de résultat. Les essais de rendu sont complétés par des captures dans plusieurs langues et tailles de fenêtre.

## Mise à jour 0.1.1

Un test de présentation rouge (fonctions absentes) a précédé la séparation entre résultat et informations indisponibles. Les contrats Linux ont d’abord échoué sur le module absent ; un test ultérieur a échoué sur l’absence de confirmation de préparation. Les validations couvrent ensuite le blocage firmware/GRUB, la confirmation avant redémarrage, et les écritures USB sans écrasement. Les algorithmes du composant GPL sont vérifiés par empreintes et réellement démarrés en VM BIOS/UEFI.

## Mise à jour 0.1.2

Les contrats du moniteur ont échoué sur le module et les nouveaux champs de préférences absents avant leur implémentation. Ils vérifient ensuite les anciennes préférences, les pourcentages inconnus/invalides, deux prélèvements réels et le classement des dix premiers sans limiter l’instantané complet. Le rendu natif distingue une absence de mesure de 0 %.

Le scénario Playwright exécute deux relances réelles, compare leurs paramètres et identifiants, puis vérifie la conservation de l’original, les cases indépendantes et les lignes cliquables. Les fixtures USB/Secure Boot concernent uniquement les capacités et les erreurs de transport ; aucune écriture USB ni préparation de démarrage n’est exécutée. La suite existante des 25 langues a détecté un formulaire remplacé par un ancien résultat ; l’état a été corrigé pour distinguer un lancement courant d’un rapport ancien. Les captures ont conduit à corriger le contraste des nouvelles cartes sombres.

Le contrôle des processus réels a ensuite détecté les TID Linux exposés par défaut par sysinfo comme des processus distincts. Un test rouge compare les PID au Tgid réel de `/proc`; la collecte désactive désormais les tâches pour éviter de répéter la mémoire et les E/S d’un même processus. Un autre cycle rouge/vert vérifie le cumul de VRAM d’un PID présent sur plusieurs GPU NVIDIA.

## Mise à jour 0.1.3

Le contrat du style a d’abord échoué sur le champ `tray_style` absent ; la migration choisit maintenant `icon_text` et refuse un style inconnu. Les tests du rendu couvrent les trois styles, les seuils 65/90 % et l’absence de confusion entre valeur inconnue et zéro. Le protocole Linux est vérifié dans un vrai bus privé avec un watcher de test : plusieurs transitions Active/Passive conservent l’objet, les propriétés se lisent sur D-Bus et Activate ouvre le panneau. Le contrôle natif ajoute trois cycles RAM et deux cycles globaux, une activation DBusMenu et la récupération PNG de l’icône de l’application réelle. Le watcher et le fichier desktop de test ne fournissent aucune mesure matérielle simulée.

Les captures Playwright vérifient les trois choix persistants, les panneaux RAM/CPU dans les deux thèmes, les liens avec icônes, le composant Panel du bloc de confidentialité et l’espace restant entre les éléments du menu latéral au survol.
