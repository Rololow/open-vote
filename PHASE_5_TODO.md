````markdown
# To-Do List - Phase 5 : Transactions Blockchain Enrichies

**Objectif :** Introduire et stabiliser les nouveaux types de transactions (IdentityValidated, ProposalCreated, SupportAdded, LawPromoted, AnonymousVote/Support) et la logique protocol-level (promotion automatique).

## Tâches proposées
- [ ] 5.1 Définir et versionner les nouveaux types de transaction
- [ ] 5.2 Ajouter logique de promotion automatique au niveau nœud/protocol
- [ ] 5.3 Tests d'intégration multi-nœuds (consensus, propagation, forks)
- [ ] 5.4 Performance & mempool hygiene (anonymous txs)
- [ ] 5.5 Audit sécurité des scripts de minage et inclusion

## Critères de réussite
- Promotion automatique s'exécute correctement dans des scénarios de test (seuils simulés)
- Transactions anonymes vérifient & n'introduisent pas de réentrées exploitable

````