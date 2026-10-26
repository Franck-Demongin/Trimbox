# Intégration de `slint-docs-mcp` avec Qwen Code dans un projet Rust

Ce document explique comment ajouter le serveur `slint-docs-mcp` à un projet Rust utilisant Slint,
et comment demander à l'assistant de s'appuyer dessus avant de modifier l'interface.

## Prérequis

- Python 3.13+ et `uv` installés sur le poste de développement.
- `git` disponible (le serveur récupère le dépôt `slint-ui/slint` au besoin).
- Une copie ou un clone stable du dépôt `slint-docs-mcp` (chemin absolu connu).
- La version de Slint utilisée par le projet, relevée dans `Cargo.lock`.

## Principe d'isolation

Le serveur tourne **en dehors** du projet Rust. Les chemins de configuration et d'index se résolvent
par rapport à la racine du serveur, jamais par rapport au répertoire courant. Qwen Code peut donc être
lancé depuis n'importe quel dossier du projet Rust sans casser le serveur.

## 1. Configuration serveur (une fois)

Dans la copie du serveur :

```bash
cp .env.example .env
```

Réglages importants dans `.env` :

- `SLINT_DOCS_VERSION` : version Slint à documenter (`1` recommandé). Ne pas mettre `latest` sauf besoin explicite.
- `SLINT_DOCS_INDEX_PATH=.local/slint-docs-index.sqlite3`
- `SLINT_DOCS_CACHE_PATH=.local/cache`
- `MCP_TRANSPORT=stdio`

Le premier appel d'un tool construit l'index automatiquement si la base locale est absente
(récupération réseau + parsing). Les lancements suivants réutilisent l'index.

## 2. Déclaration dans Qwen Code

Dans les paramètres Qwen Code (utilisateur ou projet), ajouter le serveur avec `uv --directory`
pour qu'il ne dépende pas du répertoire courant :

```json
{
  "mcpServers": {
    "slint-docs": {
      "command": "uv",
      "args": [
        "run",
        "--directory",
        "/chemin/absolu/vers/slint-docs-mcp",
        "python",
        "-m",
        "slint_docs_mcp",
        "--transport",
        "stdio"
      ],
      "env": {
        "SLINT_DOCS_VERSION": "1"
      }
    }
  }
}
```

Les tools sont alors visibles sous la forme `mcp__slint-docs__<nom>` :

- `mcp__slint-docs__slint_docs_search`
- `mcp__slint-docs__slint_docs_get_page`
- `mcp__slint-docs__slint_docs_list`
- `mcp__slint-docs__slint_reference_describe`

Vérification : `/mcp` doit lister le serveur et ses outils.

## 3. Version Slint alignée sur `Cargo.lock`

Le serveur doit documenter la même version que celle dont dépend le projet. Relever la version dans `Cargo.lock`
du projet Rust et l'injecter au lancement (`SLINT_DOCS_VERSION` dans les `env` du serveur). Par exemple,
si `Cargo.lock` contient `slint 1.18.x`, utiliser `1`. Ne pas présenter des APIs d'une version future
sans avertissement.

## 4. Règles à placer dans le `QWEN.md` du projet Rust

Ajouter une section comme celle-ci dans le `QWEN.md` du projet consommateur :

```markdown
## Documentation Slint (via slint-docs)

- Avant toute modification d'un fichier `.slint`, appeler le serveur `slint-docs` :
  1. `slint_docs_search` avec un mot-clé précis ;
  2. `slint_reference_describe` avant d'utiliser ou modifier un widget/élément/type ;
  3. `slint_docs_get_page` pour le détail d'une page.
- La version Slint du projet est celle de `Cargo.lock`. Ne pas intégrer d'API absente de cette version.
- Vérifier `page_url` pour croiser avec la documentation publiée.
- Conserver les tests UI existants et les exécuter après modification.
```

Ces règles viennent du projet, pas du serveur : le serveur reste générique.

## 5. Agent dédié (optionnel)

Un sous-agent `.qwen/agents/slint-ui.md` peut être limité aux tools pertinents :

```yaml
---
name: slint-ui
description: Modifie l'interface Slint du projet en s'appuyant sur la documentation versionnée
tools:
  - read_file
  - edit
  - mcp__slint-docs__*
---
Avant toute modification d'un fichier .slint, consulte la documentation via le serveur slint-docs
(slint_docs_search puis slint_reference_describe). Respecte les règles du QWEN.md.
```

## 6. Workflow d'usage recommandé

1. Modifier `Cargo.lock` / dépendances Slint : mettre à jour `SLINT_DOCS_VERSION` côté serveur.
2. Avant une modification UI, lancer une recherche ciblée (nom de widget, propriété, layout).
3. Lire la fiche (`slint_reference_describe`) avant d'ajouter un widget.
4. Consulter la page complète si besoin de syntaxe ou d'exemples.
5. Vérifier la navigation de l'arborescence (`slint_docs_list`) si le terme exact est inconnu.
6. Tester le code Slint/Rust après modification.

## 7. Vérifications rapides

- le serveur démarre depuis un autre répertoire courant (chemins relatifs à la racine serveur) ;
- aucune sortie parasite sur `stdout` (les logs vont sur `stderr`) ;
- les erreurs sont exploitables (ex. `SLINT_DOC_VERSION_NOT_INDEXED`) ;
- la version Slint documentée correspond à `Cargo.lock`.

## 8. Dépannage

- **Version non indexée** : lancer l'indexation via le flux prévu (`index --version N`) ou attendre la construction automatique au premier tool call selon l'intégration.
- **Chemin courant** : si le serveur se lance depuis le projet Rust, vérifier l'usage de `uv --directory`.
- **`latest` vs version 1.x** : utiliser le tag concret de Slint et comparer.
- **Réseau** : la première construction récupère `slint-ui/slint` au tag demandé.
- **Champs mutés manuellement** : ne pas modifier `.local/` à la main ; reconstruire l'index.

## 9. Emplacements conseillés dans un projet Rust

- Conserver la configuration ci-dessus dans les paramètres Qwen Code (ou un `.qwen/settings.json` partagé par l'équipe).
- Garder un `doc/slint-ui.md` ou une section `QWEN.md` qui documente les règles d'interface.
- Versionner les fixtures/tests UI du projet (indépendants du serveur).