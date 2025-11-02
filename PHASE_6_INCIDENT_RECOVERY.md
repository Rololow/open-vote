# Phase 6: Plan de Reprise en Cas d'Incident (Incident Recovery Plan)

## Vue d'ensemble

Ce document décrit les procédures d'urgence et de reprise en cas de compromission de clé ou d'incident de sécurité majeur dans le système open-vote.

## Types d'Incidents

### 1. Compromission de Clé Issuer (Critique)

**Indicateurs:**
- Clé privée issuer exposée ou volée
- Activité suspecte sur les credentials émis
- Détection d'émission non autorisée de credentials

**Procédure de Réponse Immédiate:**

1. **Isolation (Immediate - 0-15 minutes)**
   ```bash
   # Arrêter tous les services blockchain
   docker compose down
   
   # Ou si en service local
   pkill -f blockchain-server
   ```

2. **Notification (15-30 minutes)**
   - Alerter l'équipe de sécurité
   - Documenter l'heure de détection
   - Identifier le vecteur de compromission si possible

3. **Révocation de la Clé Compromise (30-60 minutes)**
   ```bash
   # Utiliser l'outil de rotation pour révoquer la version compromise
   cargo run -p blockchain-server --bin issuer_key_tool -- \
     --revoke-version <VERSION> \
     --reason "Key compromise detected on YYYY-MM-DD"
   ```

4. **Génération d'une Nouvelle Clé (60-90 minutes)**
   ```bash
   # Générer une nouvelle clé issuer sur un système sécurisé
   cargo run -p blockchain-server --bin issuer_key_tool -- \
     --key /secure/path/issuer_ed25519_key_new.json \
     --export-jwk /secure/path/issuer_pub_new.jwk
   
   # Enregistrer la nouvelle clé dans la configuration de rotation
   # (voir section "Key Rotation Procedure" ci-dessous)
   ```

5. **Mise à Jour de la Configuration (90-120 minutes)**
   ```bash
   # Mettre à jour ALLOWED_ISSUERS_DIDS pour inclure la nouvelle clé
   export ALLOWED_ISSUERS_DIDS="did:key:NEW_KEY_DID"
   
   # Redémarrer avec la nouvelle configuration
   docker compose up -d
   ```

6. **Vérification Post-Incident (2-4 heures)**
   - Vérifier que tous les nouveaux credentials utilisent la nouvelle clé
   - Auditer les credentials émis pendant la fenêtre de compromission
   - Révoquer les credentials suspects via la revocation list

### 2. Compromission d'Identité Utilisateur (Haute Priorité)

**Indicateurs:**
- Rapport utilisateur de clé privée compromise
- Activité anormale détectée (votes multiples, etc.)
- Détection de transaction frauduleuse

**Procédure de Réponse:**

1. **Vérification (0-15 minutes)**
   - Confirmer l'identité du reporter
   - Vérifier l'activité suspecte dans les logs

2. **Révocation (15-30 minutes)**
   ```bash
   # Révoquer le commitment identity de l'utilisateur
   curl -X POST http://localhost:8080/api/admin/revoke \
     -H "Content-Type: application/json" \
     -d '{
       "identifier": "<commitment_hash>",
       "identifier_type": "commitment",
       "reason": "user_requested",
       "details": "User reported private key compromise"
     }'
   ```

3. **Notification Utilisateur (30-60 minutes)**
   - Informer l'utilisateur des étapes suivantes
   - Guider vers la génération d'une nouvelle identité
   - Expliquer le processus de ré-enregistrement

4. **Monitoring (Continu)**
   - Surveiller toute utilisation de l'identité révoquée
   - Documenter dans le security monitoring system

### 3. Détection de Paramètre Mismatch (Priorité Moyenne)

**Indicateurs:**
- Alertes de parameter_mismatch dans les logs
- Échecs de vérification ZKP ou credential
- Différences dans les merkle roots

**Procédure de Réponse:**

1. **Investigation (0-30 minutes)**
   ```bash
   # Vérifier les logs de monitoring
   curl http://localhost:8080/api/monitoring/events?severity=high
   
   # Vérifier les métriques
   curl http://localhost:8080/api/monitoring/metrics
   ```

2. **Identification de la Cause (30-120 minutes)**
   - Bug dans le code?
   - Attaque active?
   - Erreur de configuration?

3. **Correction (Variable selon la cause)**
   - Si bug: corriger et déployer patch
   - Si attaque: appliquer procédures de sécurité renforcées
   - Si config: corriger la configuration et synchroniser

## Procédure de Rotation de Clé (Non-Urgence)

### Rotation Planifiée (Maintenance Régulière)

**Fréquence Recommandée:** Tous les 6-12 mois

**Étapes:**

1. **Préparation (J-7)**
   ```bash
   # Générer la nouvelle clé à l'avance
   cargo run -p blockchain-server --bin issuer_key_tool -- \
     --key issuer_ed25519_key_v2.json \
     --export-jwk issuer_pub_v2.jwk
   
   # Enregistrer dans le système de rotation
   # Éditer rotation_config.json pour ajouter la version 2
   ```

2. **Notification (J-3)**
   - Informer les opérateurs
   - Planifier la fenêtre de maintenance
   - Préparer les configurations

3. **Exécution (J-Day)**
   ```bash
   # Mettre à jour ALLOWED_ISSUERS_DIDS pour inclure les deux versions
   export ALLOWED_ISSUERS_DIDS="did:key:OLD_KEY,did:key:NEW_KEY"
   
   # Redémarrer avec la nouvelle configuration active
   # La nouvelle version sera utilisée pour les nouveaux credentials
   # L'ancienne version reste valide pour les credentials existants
   ```

4. **Période de Coexistence (J+1 à J+90)**
   - Les deux clés sont acceptées pour vérification
   - Seule la nouvelle clé signe de nouveaux credentials
   - Surveiller les métriques

5. **Retrait de l'Ancienne Clé (J+90)**
   ```bash
   # Retirer l'ancienne clé de ALLOWED_ISSUERS_DIDS
   export ALLOWED_ISSUERS_DIDS="did:key:NEW_KEY"
   
   # Note: Les credentials émis avec l'ancienne clé 
   # expireront naturellement (validity_days)
   ```

## Monitoring et Détection

### Métriques à Surveiller en Continu

1. **Taux d'Échec de Vérification**
   ```bash
   # Seuil d'alerte: > 5% d'échecs
   curl http://localhost:8080/api/monitoring/metrics | jq '.failed_verifications / .total_verifications'
   ```

2. **Parameter Mismatches**
   ```bash
   # Seuil d'alerte: > 0 dans une période de 1h
   curl http://localhost:8080/api/monitoring/events?type=parameter_mismatch&since=1h
   ```

3. **Révocations Actives**
   ```bash
   # Surveiller le nombre de révocations
   curl http://localhost:8080/api/revocation/statistics
   ```

### Alertes Automatiques

**Configuration recommandée pour production:**

```bash
# Variables d'environnement pour alerting
export ALERT_EMAIL="security@example.com"
export ALERT_WEBHOOK="https://slack.webhook.url"
export ALERT_THRESHOLD_VERIFICATION_FAILURES=50  # par heure
export ALERT_THRESHOLD_PARAMETER_MISMATCHES=1    # immédiat
```

## Tests de Reprise (Disaster Recovery Drills)

### Fréquence Recommandée: Trimestrielle

**Scénario de Test 1: Compromission Simulée**
1. Générer une clé test
2. Émettre quelques credentials test
3. Simuler la compromission
4. Exécuter la procédure de rotation d'urgence
5. Vérifier que les nouveaux credentials utilisent la nouvelle clé
6. Mesurer le temps de reprise (objectif: < 2 heures)

**Scénario de Test 2: Perte de Base de Données**
1. Sauvegarder la base actuelle
2. Simuler la perte (supprimer le fichier DB)
3. Restaurer depuis la sauvegarde
4. Vérifier l'intégrité des données
5. Mesurer le temps de reprise (objectif: < 30 minutes)

## Contacts d'Urgence

**À définir par l'organisation:**

- Security Lead: [Contact]
- DevOps On-Call: [Contact]
- Legal/Compliance: [Contact]
- Executive Sponsor: [Contact]

## Checklist Post-Incident

Après résolution d'un incident:

- [ ] Documenter chronologie complète de l'incident
- [ ] Identifier la cause racine
- [ ] Implémenter les corrections permanentes
- [ ] Mettre à jour les procédures si nécessaire
- [ ] Former l'équipe sur les leçons apprises
- [ ] Tester les corrections dans un environnement de test
- [ ] Mettre à jour la documentation
- [ ] Communiquer aux stakeholders

## Annexe: Commandes Utiles

### Vérifier l'État du Système
```bash
# Santé générale
curl http://localhost:8080/health

# Métriques de monitoring
curl http://localhost:8080/api/monitoring/metrics

# Événements récents
curl http://localhost:8080/api/monitoring/events?limit=100

# Statistiques de révocation
curl http://localhost:8080/api/revocation/statistics
```

### Backup et Restauration
```bash
# Backup de la base de données
sqlite3 ./data/blockchain.db ".backup './backups/blockchain_$(date +%Y%m%d_%H%M%S).db'"

# Backup de la configuration de rotation
cp ./data/rotation_config.json ./backups/rotation_config_$(date +%Y%m%d_%H%M%S).json

# Restauration
cp ./backups/blockchain_YYYYMMDD_HHMMSS.db ./data/blockchain.db
cp ./backups/rotation_config_YYYYMMDD_HHMMSS.json ./data/rotation_config.json
```

### Logs et Diagnostics
```bash
# Suivre les logs en temps réel
docker logs -f blockchain-rust

# Rechercher des événements spécifiques
docker logs blockchain-rust 2>&1 | grep -i "security\|error\|revok"

# Extraire les métriques
docker logs blockchain-rust 2>&1 | grep "monitoring" > monitoring_logs.txt
```

## Conclusion

Ce plan doit être:
- Revu et mis à jour trimestriellement
- Testé régulièrement (drills)
- Accessible 24/7 à l'équipe de sécurité
- Maintenu à jour avec les évolutions du système

**Dernière révision:** $(date +%Y-%m-%d)  
**Prochaine révision prévue:** [À planifier]
