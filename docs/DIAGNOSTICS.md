# Méthodologie

Computy conserve les observations, leur source, leurs paramètres et leurs limites. Il ne transforme pas une donnée manquante en résultat positif et n’attribue pas une panne à une barrette précise sur la seule base d’une erreur mémoire.

## Mémoire

Le moteur Rust réserve une allocation bornée, tente `mlock` / `VirtualLock` et indique si le verrouillage a réussi. La taille demandée ne peut dépasser 2048 Mio, un tiers de la mémoire disponible ni un quart de la mémoire visible. Une demande excessive est refusée.

Chaque passe applique cinq familles de motifs : adresses virtuelles, zéros/uns, damier, séquence pseudo-aléatoire déterministe et bit mobile selon l’index du mot. Pour chaque motif : remplissage, vérification croissante avec inversion, puis vérification décroissante avec restauration. Les lectures et écritures volatiles empêchent le compilateur d’éliminer les accès. Les divergences sont comptées ; le premier décalage, la valeur attendue et la valeur lue sont conservés.

Ce moteur est un contrôle en espace utilisateur. Il ne remappe pas toute la mémoire physique, ne désactive pas les caches, ne réalise pas un test Rowhammer et ne constitue pas une implémentation complète de Memtest86+. Sans verrouillage, des pages peuvent passer par le swap. Le motif de bits mobiles n’exerce pas les 64 positions de chaque mot pendant une seule passe.

Les compteurs EDAC accessibles sous Linux sont des compteurs depuis le démarrage. Ils ne sont pas présentés comme des erreurs exclusivement causées par le diagnostic courant.

Sources : [algorithmes et limites de Memtest86+](https://github.com/memtest86plus/memtest86plus#memory-testing-philosophy), [pannes intermittentes et durée de MemTest86](https://www.memtest86.com/tech_execution-time.html), [EDAC du noyau Linux](https://www.kernel.org/doc/html/latest/driver-api/edac.html).

## Stockage

`smartctl --scan --json` énumère les périphériques, puis `--all --json --nocheck=standby,0` lit leur télémétrie sans réveiller volontairement un disque en veille. Les appels ont un délai maximal, n’utilisent pas de shell et sont limités à 16 périphériques. Un outil absent, un accès refusé ou un périphérique non exposé reste indisponible.

Le code de sortie de smartctl est un **masque de bits**. Les alertes d’état, erreurs de média NVMe, alertes critiques, secteurs ATA en attente/non corrigibles, secteurs réalloués et journaux historiques sont interprétés séparément. Une erreur de transport ne doit pas masquer une alerte matérielle explicite. Les valeurs brutes pertinentes restent dans le rapport.

Le contrôle de fichier crée un fichier unique avec `create_new`, écrit des blocs déterministes de 1 Mio différents selon leur index, appelle `sync_all`, ferme puis rouvre le fichier et compare chaque octet relu. Sous Linux, une éviction de cache est demandée à titre consultatif. Le fichier est supprimé sur réussite, erreur ou annulation ; un échec de suppression est signalé. Une réserve de 256 Mio d’espace libre est conservée.

Le contrôle concerne le **volume des données de Computy**. Il ne parcourt pas tous les secteurs ni tous les disques et ne constitue pas un benchmark de débit. Les temps incluent génération de motifs, synchronisation et comparaison. Il ne lance pas un auto-test firmware SMART long et ne remplace pas un diagnostic constructeur. Aucune écriture brute, aucun formatage et aucun accès aux fichiers personnels ne sont nécessaires.

Source : [manuel officiel de smartctl](https://github.com/smartmontools/smartmontools/blob/master/smartmontools/smartctl.8.in).

## Processeur et capteurs

Jusqu’à quatre threads calculent le SHA-256 d’un million de caractères `a` et une somme entière de 0 à 9999. Chaque réponse est comparée à une valeur fixe connue ; le nombre de calculs terminés et les divergences sont enregistrés. La durée est bornée de 1 à 60 secondes. Cela n’exerce pas toutes les instructions, le GPU, ni la stabilité thermique prolongée.

Les températures sont des relevés ponctuels des capteurs exposés par le système. Une température ou un seuil critique absent n’est pas estimé.

## Inventaire et remplacement

Linux lit sysinfo et SMBIOS via dmidecode lorsqu’il est accessible. Windows lit les classes CIM de mémoire ; macOS utilise system_profiler. Les capacités maximales sont des **déclarations du firmware**, à confirmer avec le fabricant. `MaxCapacity` et `MaxCapacityEx` Windows sont convertis depuis les Kio.

Le débit nominal d’une barrette et son débit configuré ne prouvent pas le débit maximal de la plateforme. Celui-ci dépend aussi du CPU, de la carte mère et du firmware. Les débits DDR sont affichés en MT/s. Un format DIMM/SODIMM suggère un module remplaçable ; l’accès physique doit être confirmé dans le manuel. Un format inconnu reste inconnu. Apple Silicon utilise une mémoire intégrée non remplaçable.

Sources : [Win32_PhysicalMemory](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-physicalmemory), [Win32_PhysicalMemoryArray](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-physicalmemoryarray), [Apple Diagnostics](https://support.apple.com/fr-fr/102550).

## Notes, couverture et historique

| Observation | Note du contrôle |
|---|---:|
| Aucun défaut observé par le contrôle effectué | 10 |
| Avertissement exploitable : usure ou historique à examiner | 5 |
| Divergence ou alerte matérielle détectée | 0 |
| Indisponible, non exécuté ou interrompu | Aucune |

Le résumé utilise la note la plus basse des contrôles exploitables. Une anomalie ne peut pas être diluée dans une moyenne. Les contrôles indisponibles réduisent la couverture et produisent un bilan à compléter. Un diagnostic interrompu n’a pas de note globale positive. **La couverture est un nombre de contrôles exploitables, pas un pourcentage de RAM ou de secteurs physiquement certifiés.**

La comparaison conserve les instantanés et avertit si les machines, sources, paramètres ou conditions diffèrent. Une augmentation de capacité ou de vitesse n’est pas une preuve de meilleure santé. Les rapports Windows au démarrage ont une source distincte ; l’allocation et la durée inconnues ne sont pas inventées.
