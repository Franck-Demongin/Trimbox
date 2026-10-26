# Cahier des charges — Trimbox

## 1. Objet

Trimbox est une application de ligne de commande et d'interface graphique dont l'objectif est de lire, valider,
mettre en conformité et transformer des documents PDF, avec une spécialisation PDF/X-4. La première version doit
permettre de produire des documents PDF/X-4 valides, accompagnés d'un rapport complet d'exécution.

## 2. Périmètre

Dans le périmètre :

- Validation d'un document PDF au regard de PDF/X-4.
- Normalisation d'un document existant vers PDF/X-4.
- Gestion et validation des boîtes de page (`MediaBox`, `TrimBox`, `BleedBox`, `CropBox`, `ArtBox` selon présence).
- Recadrage central d'un document, sans traits de coupe et sans marges.
- Traitement des documents déjà conformes PDF/X, des PDF génériques et des images raster.
- Préservation ou traduction explicite des informations couleur, du profil ICC et des métadonnées.
- Rapport d'exécution précis et code de retour utilisable en script.
- Interface graphique complémentaire à la ligne de commande.

Hors périmètre initial :

- Impression, imposition avancée et gestion de pages complexes.
- Gestion technique approfondie des polices, calques et annotations au-delà de la conservation et du rapport.
- Traitement récursif de dossiers entiers ou de lots de plusieurs fichiers.
- Modification utilisateur de faible niveau des objets PDF internes.

## 3. Entrées

Les entrées acceptées comprennent :

- Documents conformes ou non conformes PDF/X.
- PDF génériques.
- Images raster à transformer en document PDF/X-4 par le traitement normal.
- Métadonnées et profils couleur présents dans la source.

Scénarios simples ciblés par la première version :

- Documents simples, généralement une à quelques pages.
- Scan ou image raster déjà préparée pour l'impression.
- PDF provenant d'outils professionnels ou génériques.

## 4. Sorties

Les sorties sont toujours les suivantes :

1. Un fichier PDF généré dont le chemin est fourni explicitement par l'utilisateur.
2. Un rapport complet au format textuel, sur la ligne de commande et dans l'interface graphique.
3. Un code de retour représentant la gravité de la situation.

Contraintes de la sortie PDF :

- Le PDF généré doit être destiné à la conformité PDF/X-4 selon la spécification de référence.
- Les dimensions demandées doivent être exactes.
- Aucun trait de coupe ni marge volontaire n'est ajouté.
- Les informations CMJN et les profils ICC sont conservés quand ils existent dans la source.

Pour un document RGB ou sans profil ICC, Trimbox adopte une solution de repli explicitement indiquée dans le rapport.
La situation ne doit pas être masquée comme une réussite normale.

## 5. Spécification de référence

La vérification de la conformité doit utiliser, comme référence principale, la spécification PDF/X-4. Les tests doivent
s'appuyer sur :

- Des documents synthétiques.
- Des fixtures versionnées dans le dépôt.
- Des documents PDF/X-4 reconnus quand ils sont disponibles pendant le développement.

## 6. Exigences fonctionnelles

### Exigences génériques

- RF-001 : Trimbox doit pouvoir lire un document PDF encodé dans un fichier.
- RF-002 : Trimbox doit pouvoir sérialiser un document modifié sans perte inattendue.
- RF-003 : Trimbox doit produire un fichier PDF/X-4 à partir d'une entrée validée ou normalisée.
- RF-004 : Trimbox doit produire un rapport d'exécution complet.

### Commande de recadrage

- RF-010 : Trimbox doit proposer une opération de recadrage (crop) central.
- RF-011 : L'utilisateur indique une largeur finale en millimètres.
- RF-012 : L'utilisateur indique une hauteur finale en millimètres.
- RF-013 : L'opération recadre le contenu depuis son centre, sans déformation ni mise à l'échelle.
- RF-014 : La sortie conserve uniquement la portion demandée, provenant directement de la source.
- RF-015 : Aucun trait de coupe, repère, marge ni élément décoratif n'est ajouté.
- RF-016 : La sortie doit avoir les dimensions exactes demandées, exprimées en points PDF.
- RF-017 : Dimensions en points : conversion des centimètres selon une règle de précision cohérente.
- RF-018 : La résolution source est utilisée quand elle est lisible.
- RF-019 : À défaut de résolution source lisible, la résolution de repli est 300 DPI.
- RF-020 : Toutes les pages d'un document source multipage sont traitées avec la même géométrie de recadrage.
- RF-021 : Le fichier de sortie est écrit uniquement à un chemin fourni explicitement.
- RF-022 : L'opération ne remplace jamais silencieusement un fichier source.

### Validation et normalisation PDF/X-4

- RF-030 : Trimbox doit valider la conformité PDF/X-4 d'un document.
- RF-031 : Trimbox doit pouvoir normaliser un document vers PDF/X-4 quand c'est possible.
- RF-032 : La vérification doit porter sur la structure nécessaire à PDF/X-4, y compris les informations de sortie.
- RF-033 : Les non-conformités détectées sont rapportées de façon complète.
- RF-034 : Une entrée non conforme peut encore produire une sortie accompagnée d'avertissements, selon la meilleure estimation possible.

### Gestion des boîtes

- RF-040 : Trimbox doit lire `MediaBox`, `TrimBox`, `BleedBox`, `CropBox` et `ArtBox` quand elles sont présentes.
- RF-041 : Trimbox doit valider la cohérence des boîtes relativement à `MediaBox`.
- RF-042 : Trimbox doit rapporter les valeurs des boîtes dans des unités lisibles, notamment les points et centimètres.
- RF-043 : La première version se concentre sur la lecture et la validation ; l'édition manuelle des boîtes n'est pas obligatoire.
- RF-044 : Si une normalisation PDF/X-4 modifie les boîtes, les changements sont consignés dans le rapport.

### Préservation des caractéristiques source

- RF-050 : Les informations CMJN doivent être conservées si elles existent dans la source.
- RF-051 : Les profils ICC présents dans la source doivent être conservés.
- RF-052 : Les métadonnées source doivent être conservées autant que possible quand elles ne sont pas incompatibles avec PDF/X-4.
- RF-053 : Les données RGB ou l'absence de profil ICC déclenchent un comportement de repli explicite.
- RF-054 : Le repli doit être mentionné clairement dans le rapport.
- RF-055 : Les transformations décoratives non demandées sont interdites.

### Interface

- RF-060 : Une interface en ligne de commande doit exposer les fonctionnalités principales.
- RF-061 : Une interface graphique optionnelle doit exposer les mêmes fonctionnalités principales.
- RF-062 : L'interface graphique est désactivable à la compilation derrière la fonction `gui`.
- RF-063 : L'expérience en ligne de commande est optimisée pour l'automatisation par script.
- RF-064 : L'interface graphique expose le minimum d'actions nécessaires sans style graphique superflu.

## 7. Sémantique des erreurs et avertissements

- Les erreurs provenant du chargement ou de l'écriture d'un fichier empêchent de produire une sortie fiable.
- Les non-conformités PDF/X-4 qui ne rendent pas l'opération impossible sont traitées comme des avertissements.
- Les erreurs structurelles immédiatement irrécupérables peuvent bloquer une normalisation.
- Le rapport final indique au minimum :
  - Le nom du fichier source.
  - Le nom du fichier généré.
  - L'opération demandée.
  - Les dimensions demandées et effectives.
  - La résolution utilisée et son origine.
  - Les boîtes lues ou modifiées.
  - Les profils ICC conservés ou transformés.
  - Les indications couleur, dont CMJN et RGB.
  - Les écarts PDF/X-4 avec leur gravité.
  - Le comportement de repli appliqué.
  - Le résultat global de l'opération.

## 8. Tests et recette

Les tests automatiques doivent couvrir :

- Le chargement et la sauvegarde d'un document PDF vide ou invalide.
- La conversion exacte des centimètres en points PDF.
- Le recadrage central avec une source de résolutions connues.
- Le repli à 300 DPI quand la résolution source est illisible.
- La préservation des couleurs CMJN et du profil ICC.
- La transformation documentée des cas RGB sans profil.
- La validation des boîtes de page.
- Le rapport généré et son code de retour.
- Le fonctionnement multi-page d'un document source.
- La non-régression du binaire sans interface graphique et avec interface graphique.

Fixtures requises :

- PDF construit pour le test dont les dimensions et la résolution sont connues.
- Image raster avec profil ICC CMJN.
- Image raster sans profil ICC.
- PDF générique non conforme PDF/X-4.
- Document PDF/X-4 déjà conforme si disponible.
- Document multipage pour le recadrage.

## 9. Contraintes techniques

- Le cœur métier devra rester indépendant de l'interface utilisateur.
- L'interface Slint reste optionnelle derrière la fonction `gui`.
- La compilation doit pouvoir se faire sans dépendance Slint.
- Le projet doit continuer à compiler sur Linux et Windows.
- Les rapports techniques doivent être testables et stables.

## 10. Hypothèses

- La première version doit prouver la valeur en traitement de documents simples.
- Les comportements de repli doivent être clairement signalés au lieu d'être masqués.
- Les dimensions finales sont censées tenir dans la source pour une opération de recadrage réussie.
