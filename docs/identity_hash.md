# Canonisation & Hash d'Engagement (Phase 2)

Objectif : produire un hash stable (sha256) représentant le contenu logique d'un Verifiable Credential citoyen, indépendamment de l'ordre des champs JSON.

## Algorithme Actuel (Implémenté)
1. Parse JSON via `serde_json`.
2. Canonisation récursive :
   - Objets : tri lexicographique des clés, concaténation `{k1:val1,k2:val2,...}` sans espaces.
   - Tableaux : ordre conservé (aucun tri) → important pour signatures futures.
   - Valeurs primitives : sérialisation standard `to_string()` de `serde_json`.
3. `sha256(canonical_json_bytes)` → `[u8;32]`.
4. Encodage hex si besoin pour stockage (`commitment_hash`).

## Exemple
Entrées équivalentes :
```
{"issuer":"did:key:z","credentialSubject":{"id":"did:key:subj"}}
{"credentialSubject":{"id":"did:key:subj"},"issuer":"did:key:z"}
```
Canonique :
```
{"credentialSubject":{"id":"did:key:subj"},"issuer":"did:key:z"}
```
Hash (hex) = `sha256(canonique)`.

## Propriétés
- Idempotent : réordonnancement des clés objet → même hash.
- Pure : aucune mutation hors fonction.
- Stable tant que logique de tri & sérialisation identiques.

## Limitations Connues
- Pas de normalisation JSON-LD complète (pas de contexte développé) — acceptable Phase 2.
- Les champs de preuve (`proof`) ne sont pas encore exclus explicitement (future étape : retirer avant hash si présents).
- Pas de mitigation collision logique (hash standard, dépend du SHA-256).

## Futur (Phase 3 / ZKP)
- Exclusion normative de champs volatils (`issuanceDate`, `expirationDate` si non nécessaires).
- Passage à JCS (JSON Canonicalization Scheme) via `serde_jcs` pour interop.
- Ajout d'un Merkle tree des commitments pour preuves d'appartenance.

## API Publiques (common::identity)
- `canonical_json_str(raw: &str) -> Result<String>`
- `compute_credential_hash(wrapper: &CitizenCredentialWrapper) -> Result<[u8;32]>`
- `hash_hex(&[u8;32]) -> String`

## Tests
- `hash_stable_reordered` : garantit stabilité si inversion ordre.
- À ajouter : test golden sur un JSON exemple sérialisé dans ce dossier.

## Benchmarks (Baseline Phase 2 - Part 1.12)
Mesures réalisées avec `cargo bench -p common --features identity` (Windows, profil `bench`, 100 échantillons Criterion) sur l'exemple VC de taille moyenne `SAMPLE_VC` :

| Opération | Temps moyen approx | Intervalle (min–max observé) |
|-----------|--------------------|-------------------------------|
| Canonisation (`identity_canonical_json`) | ~7.46 µs | 7.28 – 7.69 µs |
| Hash complet (`identity_hash_compute`) | ~7.51 µs | 7.42 – 7.62 µs |

Notes :
- Gnuplot absent → backend plotters utilisé (rapports HTML toujours générés).
- Outliers élevés (5–11 %) → accepter pour baseline initiale; prochain affinage avec `--sample-size 300` si besoin.
- Objectif Phase 2 (< 1 ms) largement respecté (marge x100). marge pour future normalisation JSON-LD ou JCS.

## Recommandations
- Toujours stocker seulement le hash + métadonnées minimales.
- Ne jamais logger le VC complet (utiliser hash pour corrélation).

---
Document de référence interne — Phase 2.