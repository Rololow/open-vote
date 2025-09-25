# Scripts de lancement des nœuds blockchain

## 1. Compilation en mode release

PowerShell:

```powershell
./scripts/build_release.ps1
```

Batch:

```bat
scripts\run_nodes.bat -Build
```

Ou directement avec cargo:

```powershell
cargo build --release -p blockchain-server
```

## 2. Lancement de plusieurs nœuds

### PowerShell (contrôle avancé + logs)

```powershell
# Compiler et lancer 3 nœuds
autres params: -Start 2 -Count 2 pour démarrer node2 et node3
./scripts/run_nodes.ps1 -Build -Start 1 -Count 3
```

Le script PowerShell détecte automatiquement le binaire dans cet ordre:
1. `target/release/`
2. `target/x86_64-pc-windows-msvc/release/`
3. (fallback debug) `target/debug/` puis `target/x86_64-pc-windows-msvc/debug/`

Logs dans `logs/nodeX.log`.

### Batch (simple)

```bat
scripts\run_nodes.bat -Build
```

Ouvre une fenêtre par nœud.

## Paramètres des nœuds
Les fichiers `node1-config.json`, `node2-config.json`, `node3-config.json` contrôlent ports et bases SQLite.

## Arrêt
Dans PowerShell: Ctrl+C arrête tous les processus.
Dans le script batch: fermer les fenêtres ouvertes.
