# Analyse critique du code ZKP pour vote anonyme et unique sur blockchain

## Objectif du projet
L'objectif est de permettre à chaque utilisateur de voter une seule fois, en s'identifiant via sa carte d'identité, tout en garantissant l'anonymat du vote sur une blockchain. Pour cela, un Zero-Knowledge Proof (ZKP) est utilisé pour prouver l'appartenance sans révéler l'identité.

## Points forts du code
- **Utilisation de Groth16 et Poseidon** : Le code utilise des primitives cryptographiques modernes (Groth16, Poseidon) adaptées aux blockchains et ZKP.
- **Séparation des entrées publiques et privées** : Les inputs du circuit sont bien séparés (root, nullifier, scope_hash publics; leaf, secret, merkle_path privés).
- **Gestion du Merkle path** : Le circuit vérifie l'appartenance à l'arbre Merkle, ce qui est standard pour prouver l'inclusion d'un élément sans révéler lequel.
- **Nullifier pour vote unique** : Le nullifier est calculé à partir du scope_hash et du secret, ce qui permet de détecter les doubles votes sans révéler l'identité.
- **Paramétrage flexible** : Les paramètres Poseidon sont injectés et sauvegardés, ce qui permet de réutiliser la configuration.

## Problèmes et limites identifiés
1. **Lien entre carte d'identité et secret**
   - Le code ne montre pas comment le secret est dérivé de la carte d'identité. Si le secret est mal généré ou non lié à l'identité, un utilisateur pourrait créer plusieurs secrets et voter plusieurs fois.
   - Il faut que le secret soit unique et dérivable uniquement à partir de la carte d'identité (ex : hash de la carte + sel).

2. **Gestion du nullifier**
   - Le nullifier est censé empêcher le double vote. Mais si le secret n'est pas bien lié à l'identité, l'utilisateur peut générer plusieurs nullifiers.
   - Il faut que le nullifier soit calculé de façon à ce qu'il soit unique pour chaque carte d'identité et scope de vote.

3. **Anonymat vs Unicité**
   - Le système doit garantir que le nullifier ne permette pas de retrouver l'identité, mais soit unique pour chaque votant. Il faut vérifier que la construction du nullifier ne fuite pas d'information sur l'identité.

4. **Vérification du circuit**
   - Le circuit vérifie bien l'appartenance à l'arbre Merkle et la cohérence du nullifier, mais il n'y a pas de vérification explicite que le leaf correspond à une carte d'identité valide.
   - Il faudrait ajouter une étape de vérification que le leaf est bien dérivé de la carte d'identité (ex : hash de la carte).

5. **Sécurité du stockage des clés et paramètres**
   - Les clés et paramètres sont stockés sur disque. Il faut s'assurer qu'ils sont bien protégés et que la génération est faite de façon sécurisée.

6. **Tests et diagnostics**
   - Les tests sont présents mais il manque des cas de test sur la résistance au double vote et à l'anonymat.

## Recommandations
- **Lier le secret à la carte d'identité** : Générer le secret comme un hash de la carte d'identité + sel, et stocker le leaf comme hash de la carte d'identité.
- **Renforcer la construction du nullifier** : Calculer le nullifier comme Poseidon(scope_hash, hash(carte_id)), pour garantir unicité et anonymat.
- **Ajouter des tests** : Tester explicitement que le même utilisateur ne peut pas voter deux fois, et qu'il est impossible de retrouver l'identité à partir du nullifier.
- **Documenter le processus** : Ajouter des commentaires et une documentation sur la façon dont l'identité est liée au vote et au nullifier.

## Conclusion
Le code pose une bonne base pour un vote anonyme et unique sur blockchain via ZKP, mais il faut renforcer le lien entre l'identité et le secret/nullifier pour garantir l'unicité du vote sans compromettre l'anonymat. Des tests et une documentation plus poussés sont nécessaires pour valider la sécurité du système.


## Risques & Faiblesses
a) Sécurité cryptographique

⚠️ Poseidon parameters hardcodés :

Les paramètres poseidon_params(width=3) sont générés via PoseidonConfig::new avec un rc_seed fixe.

Si ces paramètres ne sont pas standardisés/publics, un attaquant pourrait exploiter des faiblesses de sécurité liées à des constants mal choisies.

⚠️ Pas de vérification de validité Merkle root :

Le circuit vérifie que le chemin de Merkle mène à leaf, mais ne contraint pas que le leaf est bien celui attendu par la logique applicative (ex. leaf = H(secret) avec clé publique dérivée).

Si l’appelant injecte un leaf arbitraire, le circuit reste valide.

⚠️ Nullifier collision possible si scope n’est pas choisi correctement (taille trop petite, entropie faible).

b) Implémentation circuit

⚠️ Pas de contraintes sur la taille/forme de l’arbre :

leaf_index_bits et leaf_index ne sont pas vérifiés → incohérence possible (index hors borne).

⚠️ Aucune contrainte d’unicité sur le nullifier :

Le circuit génère juste un hash, mais ne garantit pas son unicité au niveau global (nécessite une logique off-chain/on-chain).

c) Gestion des clés et proofs

⚠️ Serialization bincode sans vérification :

Lors du save_poseidon_params_to_file et load_poseidon_params_from_file, pas de checksum ou hash sur les fichiers → attaque possible via paramètres corrompus.

⚠️ Pas de versioning robuste des paramètres :

Tu as un champ vk_version, mais il n’est pas intégré dans les preuves (donc un prouveur pourrait fournir une preuve valide avec une ancienne version du circuit).

d) Tests

⚠️ Les tests valident surtout le chemin de Merkle et la génération de preuves, mais pas :

la robustesse face à de mauvaises entrées (leaf_index hors borne, secret invalide, scope trivial),

ni la détection d’une preuve mal formée.

4. Recommandations
a) Sécurité cryptographique

Paramètres Poseidon : utiliser un set officiel/documenté (ex. ceux de Filecoin/Zcash). Ne jamais générer à la volée.

Feuille Merkle : imposer la contrainte leaf = Poseidon(secret || scope) pour lier la preuve au secret.

Nullifier : renforcer le calcul (nullifier = Poseidon(scope || secret || leaf_index) par ex.) pour réduire les collisions.

b) Circuit & contraintes

Ajouter une contrainte que leaf_index_bits corresponde bien à leaf_index.

Vérifier que la hauteur du Merkle path est correcte (pas plus/pas moins).

Ajouter une contrainte de bornage sur scope (ex. hashé avant usage).

c) Gestion des clés

Intégrer un hash SHA256 des paramètres dans les preuves (via input public), pour empêcher l’usage de fichiers corrompus.

Lier vk_version dans les inputs publics → évite l’attaque par rollback.

d) Tests

Ajouter des tests négatifs :

Mauvais chemin Merkle → preuve rejetée.

Mauvais nullifier → preuve rejetée.

Index incohérent (leaf_index != bits).

Ajouter un test de reproductibilité → preuve générée sur une machine doit être vérifiable sur une autre avec mêmes params.

5. Conclusion

Le code est globalement propre et bien structuré avec une implémentation correcte de Groth16 + Poseidon.
Les principaux points de vigilance sont liés à la sécurité cryptographique des paramètres Poseidon, la validation incomplète des inputs dans le circuit, et la robustesse de la gestion des clés.

Avec les améliorations proposées, ce circuit peut atteindre un niveau de sécurité de production.

## Comparaison client (wallet-cli) vs serveur (blockchain-server) ZKP

### 1. Implémentation client (wallet-cli)
- Génère la preuve ZKP (membership + nullifier) à partir des inputs (root, leaf, secret, merkle_path, scope).
- Calcule le nullifier pour garantir l’unicité du vote.
- Récupère le root Merkle auprès du serveur.
- Envoie la preuve et le nullifier au serveur pour vérification.

**Problèmes potentiels côté client :**
- Génération du secret : Si le secret n’est pas bien lié à l’identité, l’utilisateur peut générer plusieurs secrets et voter plusieurs fois.
- Construction du nullifier : Si le nullifier n’est pas unique par identité/scope, risque de double vote ou collision.
- Paramètres Poseidon : Si le client utilise des paramètres différents/non officiels, la preuve peut être invalide ou vulnérable.
- Gestion du root : Si le root utilisé n’est pas à jour, la preuve peut être rejetée (root drift).
- Sécurité locale : Les clés et secrets stockés sur le client doivent être protégés contre le vol ou la fuite.

### 2. Implémentation serveur (blockchain-server)
- Vérifie la preuve ZKP reçue (membership + nullifier).
- Vérifie l’unicité du nullifier pour empêcher le double vote.
- Maintient la base de données des nullifiers et des roots Merkle.
- Expose l’API pour fournir le root et les preuves Merkle aux clients.

**Problèmes potentiels côté serveur :**
- Vérification des paramètres : Si le serveur accepte des preuves avec des paramètres Poseidon non officiels ou corrompus, risque d’attaque.
- Gestion du root : Si le serveur n’ancre pas correctement les roots ou accepte des roots obsolètes, risque de replay ou d’incohérence.
- Unicité du nullifier : Si la vérification d’unicité est mal implémentée, risque de double vote.
- Versioning des VK/paramètres : Si le serveur ne vérifie pas la version du circuit ou des paramètres, risque d’attaque par rollback.
- Sécurité de la base de données : Nullifiers et roots doivent être protégés contre la corruption ou la suppression malveillante.
- Validation des inputs : Si le serveur ne vérifie pas que le leaf correspond bien à une identité valide, risque d’acceptation de preuves frauduleuses.

### 3. Problèmes croisés et divergences
- Synchronisation des paramètres : Les deux doivent utiliser les mêmes paramètres Poseidon et VK, sinon les preuves ne seront pas vérifiables.
- Gestion des versions : Le client et le serveur doivent être synchronisés sur la version du circuit et des paramètres.
- Validation de l’identité : Le serveur doit s’assurer que le leaf correspond à une identité valide, ce que le client doit garantir lors de la génération.
- Sécurité des échanges : Les preuves transmises doivent être protégées contre la falsification ou l’interception.

### 4. Recommandations
- Ajouter des contrôles stricts sur la génération et la vérification des paramètres Poseidon.
- Lier le secret et le leaf à l’identité de façon cryptographiquement sûre.
- Synchroniser la version des circuits et des VK entre client et serveur.
- Ajouter des tests croisés pour vérifier la robustesse face à des inputs malicieux ou des tentatives de double vote.