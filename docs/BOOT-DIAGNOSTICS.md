# Test de mémoire avant démarrage

Computy distingue deux modes : les contrôles **dans la session** (profils rapide, standard ou approfondi, sans redémarrage) et le test **avant démarrage**. Ce dernier est une phase du même produit, fournie dans le même installateur ; il ne nécessite pas d’installation séparée de Memtest86+.

Le composant intégré dérive de Memtest86+ 9.00. Les algorithmes de test restent inchangés ; la présentation est adaptée à Computy. Le composant autonome est sous GPLv2, avec notices et sources correspondantes incluses dans le paquet. La [fiche du composant](../third_party/computy-memory-boot/COMPUTY.md) détaille sa provenance.

## Linux sans clé USB

L’assistant vérifie l’architecture x86-64, le firmware, Secure Boot, le composant installé et GRUB. Après confirmation dans l’application et autorisation PolicyKit, un assistant Rust root sélectionne uniquement l’entrée `computy-memory-test` pour le prochain démarrage, avec `grub-reboot`. Il vérifie la confirmation avant d’autoriser le bouton de redémarrage.

Le système démarré par défaut est conservé. GRUB doit pouvoir effacer `next_entry` après utilisation ; ce parcours est limité aux environnements de démarrage compatibles, sans abstraction LVM/RAID/chiffrement et avec le système de fichiers GRUB `ext2` (famille ext). Les autres configurations sont refusées plutôt que modifiées au hasard. Une autre sélection ponctuelle existante n’est jamais écrasée.

Le premier ajout automatique est refusé avec un défaut GRUB personnalisé susceptible de changer de position ; une entrée Computy déjà présente est préservée lors des mises à jour.

La préparation est annulable. Le redémarrage demande un accord distinct. Aucun test du développement ne programme le démarrage de la machine hôte.

**Secure Boot** : le binaire Computy modifié n’est pas signé par une autorité reconnue du firmware. Secure Boot actif, ou un état indéterminé, bloque ce lancement. Computy ne change pas les clés UEFI et ne désactive pas cette protection. Un simple fork ou une réécriture Rust ne supprime pas cette contrainte. Le test en session reste disponible.

## Linux avec clé USB

L’assistant propose les volumes **USB montés en FAT32**. Il exclut les disques contenant les points de montage système, demande le choix d’une clé puis une confirmation de la destination. L’identifiant inclut l’UUID pour éviter de confondre des clés débranchées/rebranchées.

Computy copie le moteur dans `EFI/BOOT/BOOTX64.EFI`, sans formater la clé ni remplacer un chargeur existant. Une clé avec ce fichier est refusée. Une clé d’un autre format doit être préparée séparément avec l’outil de disques du système ; cette version n’efface et ne repartitionne aucun périphérique.

Le choix de la clé au démarrage dépend du firmware ; Computy explique cette étape. Secure Boot demeure applicable. Le moteur affiche la progression, les passes et les erreurs ; Échap quitte et redémarre. F6 permet de sauvegarder un rapport sur un support FAT32 pris en charge par le moteur. Éviter plusieurs clés simultanées pour ne pas rendre le choix du support de rapport ambigu.

## Résultats et limites

Le retour dans Linux **ne prouve pas** que le test a réussi ni même qu’il s’est entièrement exécuté. Aucun score n’est inventé. Cette version ne récupère pas automatiquement les résultats du test autonome sans canal persistant exploitable ; l’assistant l’indique avant la préparation. Relever le résultat à l’écran ou enregistrer le rapport USB.

Sous Windows, Computy ouvre le diagnostic mémoire du système et importe un événement de résultat récent au retour. La réouverture de Computy à la connexion est cochée par défaut dans ce parcours, avec possibilité de la désactiver. Le réglage général de démarrage de l’application reste explicite. Sur Mac, le parcours Apple dépend du modèle et n’est pas automatisé ici.

## Validation

Le moteur a réellement démarré dans deux VM QEMU, BIOS et UEFI, sans accès aux disques du PC. Des passes des tests 0, 1 et 2 ont été exécutées pour la validation de démarrage ; il ne s’agit pas d’une certification de tous les algorithmes ni du matériel physique. Les captures et le manifeste sont dans `docs/screenshots/boot-*`.

Les tests Rust couvrent le refus de Secure Boot, des configurations GRUB non compatibles, d’une autre sélection de démarrage, des requêtes non confirmées et de clés contenant un système ou un chargeur existant. Les essais UI vérifient les accords séparés. Le parcours réel de modification de GRUB n’a pas été exécuté sur la machine hôte.

Sources : [Memtest86+ : démarrage et Secure Boot](https://memtest.org/readme), [GRUB : next_entry ponctuel](https://www.gnu.org/software/grub/manual/grub/html_node/next_005fentry.html), [licence du composant](../third_party/computy-memory-boot/LICENSE).
