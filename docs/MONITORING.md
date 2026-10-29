# Moniteur de la barre système

Computy 0.1.3 fournit des indicateurs activés par défaut pour la RAM, le CPU et le GPU. Dans **Préférences → Moniteur dans la barre système**, chaque indicateur et chaque volume peut être sélectionné, ou le moniteur entièrement désactivé. Les choix s’appliquent dans les deux secondes après enregistrement.

Trois styles sont disponibles : **icône + pourcentage** par défaut, **anneau** conservant le dessin précédent, et **icône remplie** selon l’utilisation. Les couleurs sont bleues sous 65 %, jaunes de 65 à 89 %, rouges à partir de 90 %. Les valeurs inconnues restent grises et ne deviennent pas 0 %.

Le panneau détaillé réutilise les composants de l’application : grand anneau, pourcentage, icône et nom du composant, dix processus avec leurs mesures, icônes des applications lorsqu’elles sont accessibles, et actions avec icônes. **Voir tout** ouvre les processus complets dans la fenêtre principale. Le panneau se ferme avec Échap, son bouton ou la perte de focus. Un panneau masqué ne prélève pas de télémétrie.

Sous Linux, le backend StatusNotifier garde le même objet et passe entre Active et Passive ; il se réenregistre lors du retour du serveur de la barre système. Ce mécanisme évite de recréer des AppIndicator avec le même identifiant. Le menu DBusMenu fournit également les mesures, les icônes et les actions aux hôtes qui le demandent. Certaines versions GNOME imposent ce menu au premier clic : sa première entrée, le double clic ou le clic central ouvrent le panneau. Les hôtes qui transmettent directement Activate et Windows/macOS ouvrent le panneau au clic.

Les icônes des processus sont résolues localement à partir de leur PID : applications et thème GTK sous Linux, icônes natives des exécutables et applications sous Windows/macOS. Les images sont converties en PNG et mises en cache ; un pictogramme neutre remplace une icône introuvable. Aucun chemin d’exécutable n’entre dans les instantanés CLI/MCP.

Fermer la fenêtre garde le moniteur actif si au moins une icône existe. **Quitter Computy** dans le menu termine l’application. Relancer le programme restaure la fenêtre existante, sans créer un second moniteur. Désactiver les derniers indicateurs alors que la fenêtre est masquée la réaffiche. Les mesures s’arrêtent lorsque le moniteur est désactivé, sauf lorsqu’une vue de mesures est ouverte ou qu’un client CLI/MCP les demande explicitement. Le moniteur ne lance aucun diagnostic et n’ajoute aucun bilan à l’historique.

## Sens des valeurs

| Mesure | Source et portée |
|---|---|
| RAM | Mémoire utilisée / mémoire physique visible par le système, via sysinfo. |
| CPU | Charge globale entre deux prélèvements ; le premier prélèvement reste inconnu. Par processus, la valeur est normalisée par le nombre de processeurs logiques pour représenter une part de la capacité totale. |
| GPU Linux | Plus forte utilisation disponible parmi les cartes AMD exposant `gpu_busy_percent` et les GPU NVIDIA exposés par `nvidia-smi`. Intel et les pilotes sans ces compteurs restent indisponibles. |
| GPU Windows | Compteurs WDDM via CIM : somme des processus par moteur, puis moteur le plus occupé. La disponibilité dépend du pilote et des droits. |
| GPU macOS | Compteurs d’utilisation IOAccelerator lorsqu’ils existent. Leur présence n’est pas garantie, notamment selon le modèle et la version du pilote ; aucune fréquence ou utilisation de VRAM n’est transformée en charge GPU. |
| Disque dans l’anneau | Espace occupé du volume sélectionné. Le menu donne aussi les débits de lecture et d’écriture exposés par sysinfo. Ce pourcentage n’est pas un taux d’activité ni une note de santé. |
| RAM par processus | Mémoire résidente et part de la RAM physique ; les pages partagées peuvent être comptées plusieurs fois. Les processus protégés peuvent masquer leurs compteurs. |
| Disque par processus | E/S de tous les volumes confondus. Le pourcentage est la part des E/S observables parmi les processus lisibles ; l’attribution à un volume précis n’est pas disponible. |
| GPU par processus | VRAM des **processus de calcul NVIDIA**, lorsqu’elle est fournie. Cela n’énumère pas tous les processus graphiques et ne donne pas un pourcentage de charge GPU par processus. Les autres cas restent inconnus. |

CPU/RAM/processus/volumes sont prélevés toutes les deux secondes. Les requêtes GPU sont espacées d’au moins six secondes et ont un délai maximal ; elles s’exécutent hors du thread graphique. Une donnée indisponible est représentée par `—` ou « Indisponible », jamais par un succès ou un zéro inventé. Les valeurs ne constituent pas un diagnostic de santé.

Sous Linux, la collecte exclut les threads individuels pour éviter de répéter les compteurs de leur processus.

Les noms et PID restent des données : ils ne sont jamais exécutés, les caractères de contrôle sont retirés des menus et les noms y sont raccourcis. La vue complète garde les noms d’origine. Ni arguments de lancement ni variables d’environnement des processus ne sont collectés. Aucune élévation de privilèges, installation de pilote, écriture de disque ou connexion réseau n’est utilisée par le moniteur.

## Intégration au bureau

macOS affiche les indicateurs dans la barre de menus ; Windows dans la zone de notification, éventuellement son menu des icônes masquées. Linux nécessite un hôte AppIndicator/StatusNotifier ; GNOME peut nécessiter son extension correspondante. Le placement et la taille finale des icônes dépendent du bureau. Sur Linux/macOS, le style icône + pourcentage utilise le libellé natif lorsque disponible : sa couleur de texte est imposée par le bureau, tandis que l’icône reste colorée selon le seuil. Windows utilise une version compacte dans son emplacement d’icône ; son système ne fournit pas de libellé adjacent. Les pourcentages du panneau détaillé, les anneaux et les remplissages gardent leurs couleurs. Les menus fournissent un texte complet lorsque la petite taille de la zone de notification limite la lecture de l’icône.

## CLI et MCP

```bash
computy-cli --json monitor
computy-cli --json monitor --watch
computy-cli settings set --tray-style icon_text
computy-cli settings set --tray-style ring
computy-cli settings set --tray-style fill
computy-cli settings set --tray-enabled false
computy-cli settings set --tray-enabled true --tray-components memory,cpu,gpu
computy-cli settings set --tray-disks '/,/home'
computy-cli settings set --tray-disks ''
```

`monitor --watch` émet des instantanés JSON jusqu’à Ctrl+C. L’outil MCP `computy_monitor` est accessible en lecture seule et expose le même instantané, avec les valeurs inconnues à `null`. Un premier appel initialise le compteur CPU ; le suivant doit être espacé d’au moins une seconde. Les préférences restent partagées avec l’application.

## Références et validation

Implémentation native fondée sur [Tauri System Tray](https://v2.tauri.app/learn/system-tray/), [sysinfo](https://docs.rs/sysinfo/0.37.2/sysinfo/), les [compteurs AMD documentés par le noyau](https://kernel.org/doc/html/latest/gpu/amdgpu/thermal.html), et les compteurs optionnels des pilotes. Linux est vérifié localement ; les intégrations Windows/macOS et les pilotes GPU non présents sur cette machine nécessitent encore une validation physique.
