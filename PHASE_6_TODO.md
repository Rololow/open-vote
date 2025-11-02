````markdown
# To-Do List - Phase 6 : Durcissement Opérationnel & Retrait Clé Privée Serveur

**Objectif :** Retirer toute détention de clé privée au niveau serveur pour les comptes utilisateurs, finaliser rotation de clés issuer, et durcir l'opérabilité.

## Tâches proposées
- [x] 6.1 Supprimer stockage clé privée utilisateur côté serveur
- [x] 6.2 Implémenter rotation clé issuer + support coexistence (acceptation de VK multiples)
- [x] 6.3 Politique de révocation & revocation list/accumulator
- [x] 6.4 Monitoring & alerting : mismatch params, verification failures
- [x] 6.5 Plan de reprise incident (compromise clé issuer)

## Critères de réussite
- ✅ Aucune clé privée utilisateur persistée sur serveur (confirmé)
- ✅ Procédure de rotation testée et documentée

## Status: ✅ COMPLETE

Voir PHASE_6_ACCOMPLISHMENTS.md pour les détails complets de l'implémentation.

````