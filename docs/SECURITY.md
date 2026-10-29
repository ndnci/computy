# Sécurité et accès aux agents

## Frontière d’accès

Tauri charge les fichiers locaux de l’application. Le rendu n’a ni Node.js, ni commande shell, ni accès générique aux fichiers. Les commandes Rust acceptent des méthodes connues et des paramètres bornés. La politique CSP de production limite scripts, ressources et communications à l’application et à son IPC. Les liens externes passent par une liste de domaines HTTPS et s’ouvrent dans le navigateur système.

Les sondes OS exécutent uniquement des programmes et arguments définis par le code. Aucun argument utilisateur n’est interpolé dans PowerShell ou un shell. Les sous-processus de sondage ont des délais et une sortie limitée.

L’aperçu web est un outil de développement, lié à localhost. Son adaptateur exige POST JSON, un en-tête applicatif et une origine de même hôte. Il n’est pas un serveur de production ni le transport MCP.

## MCP stdio

Le client lance la CLI localement et échange des messages JSON-RPC délimités par des lignes. `stdout` ne contient que le protocole ; les journaux vont sur `stderr`. Chaque requête est limitée à 64 Kio.

Les permissions sont choisies par le propriétaire au lancement du processus :

| Mode | Outils |
|---|---|
| Par défaut | Inventaire, historique paginé, lecture/export d’un rapport, comparaison, progression, lecture des préférences et de l’attente Windows |
| `--allow-diagnostics` | Ajouter lancement et annulation de contrôles bornés |
| `--allow-history-write` | Ajouter renommage/suppression, préférences et import d’un événement Windows réel |

Les outils non autorisés ne sont pas listés et leur invocation directe est refusée. Les outils n’exposent ni shell, ni chemin d’écriture libre, ni redémarrage, ni formatage. Les résultats, noms de périphériques et libellés de rapports sont des **données non fiables**, jamais des instructions pour l’agent.

## HTTP temporaire

- Adresse imposée : **127.0.0.1**, jamais `0.0.0.0`.
- Jeton aléatoire de 256 bits, créé pour ce processus, comparaison en temps constant.
- Fichier de jeton créé sans écrasement ; permissions Unix `0600`. Sous Windows, utiliser un répertoire privé de l’utilisateur et ses ACL.
- Durée de 60 à 3600 secondes, mesurée avec une horloge monotone ; 900 secondes par défaut.
- Jeton transmis uniquement dans `Authorization: Bearer …`, jamais dans l’URL ou le contenu d’un rapport.
- Hôte local attendu et origine vérifiés. Les origines de navigateur sont refusées sauf autorisation HTTPS exacte.
- Corps limité à 64 Kio et 5 secondes de lecture ; au plus 8 traitements simultanés et 120 requêtes authentifiées par minute.
- JSON-RPC par POST ; notifications acceptées sans réponse de protocole, réponse HTTP 202. GET retourne 405 car ce serveur n’offre pas de flux SSE.
- Le suivi des tâches passe par un identifiant de diagnostic ; les tâches sont attachées au processus serveur qui les a lancées.

Arrêter normalement la CLI ou attendre l’expiration révoque l’accès, annule le diagnostic encore actif et supprime le fichier de jeton. Après une interruption brutale du processus, un fichier peut subsister, mais son jeton ne donne accès à aucun nouveau processus : chaque lancement génère un nouveau secret.

## Tunnel

`--cloudflare-tunnel` est un choix explicite. Computy utilise uniquement un `cloudflared` déjà installé, sans téléchargement automatique, création de compte ou stockage d’identifiants Cloudflare. Le sous-processus reçoit une URL locale fixe et un en-tête Host local. Son arrêt est lié à la durée de vie du serveur.

Une URL publique n’est pas une autorisation. Le jeton reste obligatoire à travers le tunnel. Transmettre le jeton au client distant par un canal approprié, avec les permissions minimales nécessaires, puis arrêter le serveur après usage.

Ce transport personnel utilise un jeton Bearer prépartagé ; il ne fournit pas OAuth. Pour un service permanent ou plusieurs utilisateurs, ajouter une passerelle d’identité/OAuth et un tunnel géré. Les Quick Tunnels sont destinés aux usages temporaires, sans garantie de disponibilité. Aucune ouverture publique n’est nécessaire aux tests du projet.

Références : [transports MCP](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports), [autorisation MCP](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization), [bonnes pratiques MCP](https://modelcontextprotocol.io/docs/2025-11-25/tutorials/security/security_best_practices), [Quick Tunnels Cloudflare](https://developers.cloudflare.com/tunnel/get-started/quick-tunnels/).

## Données et ressources

Les identifiants de rapport n’acceptent que chiffres et tirets. Les fichiers trop volumineux, symlinks et empreintes invalides sont refusés à la lecture. L’écriture passe par un fichier temporaire unique, une synchronisation et un remplacement atomique. Une empreinte SHA-256 détecte les altérations accidentelles ; elle ne protège pas contre un attaquant possédant les droits de l’utilisateur et pouvant recalculer cette empreinte.

Les diagnostics ont des limites d’allocation et de durée. Un verrou de fichier empêche les interfaces partageant le même répertoire de données de lancer simultanément plusieurs diagnostics. Des répertoires volontairement distincts ne partagent pas ce verrou. Aucun test n’est lancé automatiquement à l’ouverture ni par le serveur MCP en lecture seule.

## Parcours avant démarrage 0.1.1

Les fonctions de préparation et de redémarrage appartiennent au parcours local explicite ; elles ne sont pas exposées au MCP. Sous Linux, le paquet installe un assistant Rust root et une politique PolicyKit `auth_admin`, sans conservation d’autorisation. L’assistant accepte uniquement `verify`, `schedule` et `cancel`, avec des chemins et commandes système fixes ; aucun chemin ni argument de commande libre n’est accepté.

Les programmes, configurations GRUB et états doivent appartenir à root et ne pas être modifiables par groupe/autres utilisateurs. Le helper verrouille l’état, refuse Secure Boot et les configurations GRUB non prises en charge, préserve toute autre sélection ponctuelle et ne confirme la préparation qu’après relecture. Un état non confirmé ne permet pas le redémarrage depuis Computy. Les métadonnées de préparation (UID, date, identifiant de démarrage) sont publiquement lisibles pour l’interface ; elles ne contiennent ni secret ni résultat RAM.

Le paquet ajoute sa propre entrée et son moteur dans `/boot` via le gestionnaire de paquets ; il ne change pas `GRUB_DEFAULT`, les clés du firmware, les partitions ou le chargeur de démarrage installé. La préparation change seulement `next_entry` via les outils GRUB standard. Le script de retrait ne supprime cette sélection que si elle désigne exactement Computy. La programmation réelle reste à valider sur une machine de test compatible ; les essais de développement n’altèrent pas le démarrage de l’hôte.

La préparation USB s’exécute avec les droits de l’utilisateur, après sélection et confirmation. Elle revalide les volumes montés, USB et FAT32, exclut le disque contenant les points de montage système, refuse les liens symboliques de destination et crée le chargeur sans écrasement. Aucun formatage ni accès en écriture à un périphérique brut n’est disponible.

## Moniteur et processus

Le moniteur utilise les droits de l’utilisateur courant. Les commandes GPU ont des paramètres fixes, aucun shell ni élévation, un délai maximal et une sortie bornée. Les noms/PID sont des données non fiables, jamais des instructions ; arguments et environnements ne sont pas collectés. Les noms sont échappés par React et nettoyés pour les menus natifs. Le moniteur est désactivable et n’exécute aucun test de charge. Le nouvel outil MCP `computy_monitor` est en lecture seule ; il expose les mesures et la liste des processus accessibles au compte exécutant Computy, y compris leurs noms, aux clients déjà autorisés.

## Panneau des indicateurs

Le panneau utilise les assets locaux de Computy et le même CSP. Son accès à `engine_call` est limité à la télémétrie et aux préférences en lecture. Ses actions ont des destinations fixes ; les noms de processus restent du texte échappé. Les icônes sont résolues à partir de PID locaux, avec 32 PID par requête, un cache borné et des PNG limités en taille ; le client ne fournit aucun chemin. Les chemins d’exécutables servant à la résolution ne sont pas envoyés à la CLI ou au MCP. Le service StatusNotifier/DBusMenu est limité au bus de session de l’utilisateur ; aucun port réseau n’est ouvert. Les tests du watcher s’exécutent dans un bus privé isolé.
