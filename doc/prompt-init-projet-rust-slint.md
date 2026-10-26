# Mission

Tu initialises un projet Rust avec une interface Slint, **dans le répertoire courant**, qui est déjà la racine du projet. Ton rôle est d'abord de cadrer le projet avec moi, puis de mettre en place une base propre, reproductible et livrable (CI GitHub Actions avec binaires Windows et Linux).

# Contexte d'exécution

- Le répertoire courant **est** le projet. Son nom formalise le nom du projet (par exemple `pdfstroke`). Détermine-le toi-même (`basename "$PWD"`), ne me le demande pas, et **ne crée aucun sous-dossier de projet**.
- Initialise avec `cargo init` **dans le répertoire courant**, jamais avec `cargo new`. Utilise `--name` si le nom du répertoire n'est pas un nom de crate valide (majuscules, tirets, chiffre initial, mot réservé…) : propose alors un nom de crate et ne renomme jamais le dossier.
- `cargo init` ne doit écraser aucun fichier existant. S'il y a conflit (README, `.gitignore`, `Cargo.toml`…), arrête-toi et demande-moi.
- Respecte l'état Git existant : si le dossier est déjà un dépôt, ne relance pas `git init` (`--vcs none` si nécessaire) et ne fais aucun commit sans que je le demande.
- Le serveur MCP de documentation Slint est déclaré sous le nom **`slint-docs`**. Ses outils apparaissent sous la forme `mcp__slint-docs__<outil>` :
  - `mcp__slint-docs__slint_docs_search`
  - `mcp__slint-docs__slint_docs_get_page`
  - `mcp__slint-docs__slint_docs_list`
  - `mcp__slint-docs__slint_reference_describe`
- Sa version par défaut est `1` (dernière version 1.x indexée). Comme le projet peut utiliser une version plus ancienne, **passe toujours explicitement le paramètre `version`** correspondant à la version réellement résolue dans `Cargo.lock`.

# Règles de conduite

1. **Phase 0 en lecture seule.** Avant de me poser des questions, fais le constat : contenu du répertoire, état Git, version de la toolchain (`rustc --version`, `cargo --version`), présence des outils MCP `slint-docs` dans ta liste d'outils. N'écris rien et n'installe rien tant que je n'ai pas répondu à la phase 1.
2. Pose tes questions **en une seule fois**, regroupées par thème et numérotées. Pour chacune, propose une **valeur par défaut recommandée** que je peux accepter d'un mot. Si un outil de question interactif est disponible, utilise-le ; sinon une liste numérotée.
3. Ne pose pas une question dont la réponse est déjà dans mon message ou dans le dépôt. Si une réponse est ambiguë, dis ce que tu comprends et demande confirmation.
4. Si je ne réponds pas à un point, applique la valeur par défaut et liste-la dans les « hypothèses » du plan.
5. **Ne devine jamais une API, une version ou une option.** Vérifie-la (documentation via `slint-docs`, recherche web, `cargo`) et dis ce que tu as vérifié.
6. Si le serveur `slint-docs` est indisponible, dis-le clairement dès la phase 0 et propose une alternative (documentation officielle via recherche web) plutôt que de continuer de mémoire.

# Phase 1 : questions à me poser

Adapte et complète cette liste selon le contexte (maximum environ 15 questions). Le nom du projet est déjà défini par le répertoire : confirme seulement le nom de crate dérivé et demande le nom d'affichage.

**Projet**
1. Nom d'affichage de l'application (celui de la fenêtre et de la documentation), si différent du nom du répertoire ?
2. Description en une ou deux phrases, et usage visé (outil personnel, public, interne) ?
3. Dépôt GitHub (`propriétaire/dépôt`) et visibilité ?
4. Licence ? Rappelle-moi les options de licence de Slint et leurs conséquences, après les avoir vérifiées dans la documentation officielle.
5. Identifiant d'application au format DNS inversé (par exemple `io.github.<utilisateur>.<App>`), utilisé sous Linux (fichier `.desktop`, `set_xdg_app_id`) ?

**Architecture**
6. Type de projet : GUI seule, GUI + CLI, bibliothèque + GUI ? L'UI doit-elle être **optionnelle** derrière un feature Cargo (par exemple `gui`) ?
7. Crate unique ou workspace ? (Par défaut : logique métier séparée de l'UI, testable sans interface.)
8. Besoins techniques : asynchrone (tokio ou autre), fichiers de configuration ou données utilisateur, réseau, persistance, internationalisation ?
9. Dépendances imposées ou interdites ?

**Slint**
10. Backend et moteur de rendu (winit, femtovg, skia, software…) : quelles features activer ? (Par défaut : `default-features = false` et seulement le nécessaire, après vérification de la documentation.)
11. Style des widgets : suivre le thème système (`std-widgets` et `Palette`) ou style imposé ? Thème clair et sombre ?
12. Intégration de l'UI : macro `slint!` ou fichiers `.slint` compilés par `slint-build` dans `build.rs` ? (Par défaut : fichiers `.slint` séparés.)
13. Taille de fenêtre préférée et minimale, accessibilité, icônes (source et format) ?

**Qualité et livraison**
14. Édition Rust et version minimale supportée (MSRV) ? (Par défaut : édition et toolchain stable les plus récentes, vérifiées.)
15. Livrables : architectures cibles (par défaut `x86_64` Linux et Windows), formats souhaités (Linux : archive `tar.gz`, AppImage, `.deb` ; Windows : archive `.zip`, installateur), déclencheur de release (par défaut : tag `v*`), checksums, notes de version ?

# Phase 2 : plan à valider

Après mes réponses, présente un plan court avant d'écrire du code : arborescence prévue dans le répertoire courant, features Cargo, décisions d'architecture, liste des fichiers de CI et de packaging, **hypothèses retenues**, points à risque. Attends mon accord.

# Phase 3 : mise en place

**Rust**
- Lance `cargo init` dans le répertoire courant (voir « Contexte d'exécution »), puis complète `Cargo.toml` : `rust-version`, `[lints]`, profil `release` adapté, métadonnées complètes.
- Structure : logique métier hors de l'UI, point d'entrée fin, erreurs typées (`thiserror` en bibliothèque, `anyhow` en binaire si pertinent), journalisation via `tracing` ou `log`.
- Application : versionne `Cargo.lock`.
- Fichiers de base, sans écraser l'existant : `.gitignore`, `README.md`, `CHANGELOG.md`, licence, `rustfmt.toml` si nécessaire, `deny.toml` (`cargo-deny`) si accepté.
- Tests unitaires de la logique ; tests d'intégration dès que possible.

**Slint**
- **Avant d'écrire ou de modifier du Slint, consulte le serveur `slint-docs`** (`slint_docs_search`, `slint_reference_describe`, `slint_docs_get_page`, `slint_docs_list`) pour chaque widget, élément ou propriété utilisé. N'invente pas d'API.
- Ajoute la dépendance Slint, lance `cargo check`, puis relève la version résolue avec `cargo tree -p slint --depth 0` (ou dans `Cargo.lock`). Utilise **cette version** pour interroger la documentation, pas la dernière. Signale tout écart notable avec la dernière version 1.x.
- Un fichier `.slint` par vue ou composant, tokens de style (espacements, rayons, tailles) centralisés dans un global, couleurs issues de `Palette` sauf état sémantique.
- Surface publique du composant racine (propriétés, callbacks) documentée et minimale ; la logique reste côté Rust.
- Appeler `slint::set_xdg_app_id` après la création de la fenêtre si pertinent, avec gestion explicite de l'erreur.
- Windows : masquer la console en release (`windows_subsystem`) et prévoir l'icône de l'exécutable.
- Linux : fichier `.desktop` et icône, si le format de livraison s'y prête.

**Contexte pour l'assistant**
- Génère un `QWEN.md` concis (architecture, commandes, conventions, module de liaison UI, rappel d'utiliser `slint-docs` avec la version du `Cargo.lock`) et propose une règle conditionnelle `.qwen/rules/ui-slint.md` limitée aux fichiers `**/*.slint`, contenant les règles de style validées.
- Demande-moi si `.qwen/` doit être versionné (si `.qwen/` est ignoré par Git, les règles ne le seront pas) et adapte le `.gitignore` en conséquence.

# Phase 3 bis : CI/CD GitHub Actions

- Workflow `ci.yml` (sur push et pull request) : `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, et compilation du feature GUI, sur Linux **et** Windows.
- Workflow `release.yml` (sur tag `v*`) : compilation en `--release` pour Linux et Windows (`x86_64`, plus `aarch64` seulement si je le demande), packaging dans les formats choisis, calcul des checksums, publication des artefacts dans une GitHub Release.
- Dépendances système de Slint sur Linux : cherche-les dans la documentation officielle de la version utilisée et installe-les dans le workflow. Explique le compromis de portabilité (version de glibc selon l'image de runner) avant de choisir.
- **Versions les plus récentes visées, mais vérifiées** : cherche les dernières versions stables des actions utilisées, de la toolchain Rust et des labels de runners avant de les écrire. Épingle les actions par version ou SHA (avec le numéro en commentaire) et ajoute un `dependabot.yml` pour les actions et les crates.
- Bonnes pratiques : cache Cargo, `permissions` minimales par job, `concurrency` pour annuler les exécutions obsolètes, noms d'artefacts explicites (nom, version, plateforme), aucun secret en clair.
- Vérifie la syntaxe des workflows (par exemple avec `actionlint` si disponible) et explique ce que tu n'as pas pu tester localement.

# Définition de terminé

- Le projet est initialisé dans le répertoire courant, sans sous-dossier, et aucun fichier existant n'a été écrasé.
- `cargo fmt --check`, `cargo clippy -- -D warnings` et `cargo test` passent ; le feature GUI compile.
- `cargo tree -p slint --depth 0` confirme la version de Slint, mentionnée dans le README.
- Les workflows sont écrits et leurs hypothèses listées.
- Rapport final : fichiers créés, décisions prises, ce qui a été vérifié et ce qui reste à valider de mon côté (première exécution de la CI, test sur Windows réel).
