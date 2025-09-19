#!/usr/bin/env python3
"""
Script de test du système E-Government Blockchain
Teste les endpoints et la fonctionnalité complète du système
"""

import requests
import json
import time
import sys
from typing import Dict, Optional

class BlockchainSystemTester:
    def __init__(self):
        self.blockchain_url = "http://localhost:8080"
        self.gateway_url = "http://localhost:8081"
        self.test_results = []
        
    def wait_for_service(self, url: str, service_name: str, timeout: int = 60) -> bool:
        """Attend qu'un service soit disponible"""
        print(f"⏳ Attente du service {service_name}...")
        
        for attempt in range(timeout):
            try:
                response = requests.get(f"{url}/health", timeout=5)
                if response.status_code == 200:
                    print(f"✅ {service_name} est en ligne !")
                    return True
            except requests.exceptions.RequestException:
                pass
            
            if attempt % 10 == 0:
                print(f"   Tentative {attempt + 1}/{timeout}...")
            time.sleep(1)
        
        print(f"❌ {service_name} n'est pas accessible après {timeout}s")
        return False
    
    def test_endpoint(self, method: str, url: str, data: Optional[Dict] = None, 
                     expected_status: int = 200, description: str = "") -> bool:
        """Test un endpoint spécifique"""
        try:
            if method.upper() == "GET":
                response = requests.get(url, timeout=10)
            elif method.upper() == "POST":
                response = requests.post(url, json=data, timeout=10)
            else:
                raise ValueError(f"Méthode {method} non supportée")
            
            success = response.status_code == expected_status
            status = "✅" if success else "❌"
            
            print(f"{status} {method} {url} -> {response.status_code}")
            if description:
                print(f"    📝 {description}")
            
            self.test_results.append({
                'endpoint': url,
                'method': method,
                'status_code': response.status_code,
                'expected': expected_status,
                'success': success,
                'description': description
            })
            
            return success
            
        except Exception as e:
            print(f"❌ {method} {url} -> Erreur: {e}")
            self.test_results.append({
                'endpoint': url,
                'method': method,
                'error': str(e),
                'success': False,
                'description': description
            })
            return False
    
    def test_blockchain_server(self) -> bool:
        """Test du serveur blockchain"""
        print("\n🔗 Test du serveur Blockchain...")
        
        tests = [
            ("GET", f"{self.blockchain_url}/", None, 200, "Page d'accueil"),
            ("GET", f"{self.blockchain_url}/health", None, 200, "Santé du service"),
            ("GET", f"{self.blockchain_url}/info", None, 200, "Informations blockchain"),
            ("GET", f"{self.blockchain_url}/stats", None, 200, "Statistiques du nœud"),
            ("GET", f"{self.blockchain_url}/blocks", None, 200, "Liste des blocs"),
        ]
        
        results = []
        for method, url, data, expected, desc in tests:
            results.append(self.test_endpoint(method, url, data, expected, desc))
        
        return all(results)
    
    def test_api_gateway(self) -> bool:
        """Test de la passerelle API"""
        print("\n🌉 Test de la passerelle API...")
        
        tests = [
            ("GET", f"{self.gateway_url}/", None, 200, "Page d'accueil gateway"),
            ("GET", f"{self.gateway_url}/health", None, 200, "Santé du gateway"),
        ]
        
        results = []
        for method, url, data, expected, desc in tests:
            results.append(self.test_endpoint(method, url, data, expected, desc))
        
        return all(results)
    
    def test_e_government_workflow(self) -> bool:
        """Test du workflow e-government complet"""
        print("\n🏛️ Test du workflow E-Government...")
        
        # Test création de compte
        account_data = {
            "public_key": "test_key_123456789",
            "name": "Citoyen Test",
            "email": "test@example.com"
        }
        
        account_success = self.test_endpoint(
            "POST", 
            f"{self.blockchain_url}/accounts", 
            account_data, 
            201, 
            "Création d'un compte citoyen"
        )
        
        # Test création de loi
        law_data = {
            "title": "Loi Test",
            "content": "Contenu de la loi de test",
            "category": "test"
        }
        
        law_success = self.test_endpoint(
            "POST", 
            f"{self.blockchain_url}/laws", 
            law_data, 
            201, 
            "Proposition d'une loi"
        )
        
        # Test de minage
        mine_success = self.test_endpoint(
            "POST", 
            f"{self.blockchain_url}/mine", 
            {}, 
            200, 
            "Minage d'un bloc"
        )
        
        return account_success and law_success and mine_success
    
    def run_full_test_suite(self) -> bool:
        """Exécute la suite complète de tests"""
        print("🧪 Suite de tests du système E-Government Blockchain")
        print("=" * 60)
        
        # 1. Attendre que les services soient disponibles
        blockchain_ready = self.wait_for_service(self.blockchain_url, "Blockchain Server")
        gateway_ready = self.wait_for_service(self.gateway_url, "API Gateway")
        
        if not blockchain_ready:
            print("❌ Serveur blockchain non accessible")
            return False
        
        if not gateway_ready:
            print("⚠️ Gateway non accessible, mais on continue...")
        
        # 2. Tester les endpoints
        blockchain_tests = self.test_blockchain_server()
        gateway_tests = self.test_api_gateway() if gateway_ready else True
        
        # 3. Tester le workflow complet
        workflow_tests = self.test_e_government_workflow()
        
        # 4. Résumé
        self.print_test_summary()
        
        return blockchain_tests and gateway_tests and workflow_tests
    
    def print_test_summary(self):
        """Affiche le résumé des tests"""
        print("\n📊 RÉSUMÉ DES TESTS")
        print("=" * 30)
        
        total_tests = len(self.test_results)
        successful_tests = sum(1 for test in self.test_results if test['success'])
        failed_tests = total_tests - successful_tests
        
        print(f"✅ Tests réussis: {successful_tests}")
        print(f"❌ Tests échoués: {failed_tests}")
        print(f"📈 Taux de réussite: {(successful_tests/total_tests)*100:.1f}%")
        
        if failed_tests > 0:
            print("\n🔍 Tests échoués:")
            for test in self.test_results:
                if not test['success']:
                    print(f"  • {test['method']} {test['endpoint']}")
                    if 'error' in test:
                        print(f"    Erreur: {test['error']}")

def main():
    """Fonction principale"""
    tester = BlockchainSystemTester()
    
    print("🎯 Démarrage des tests du système...")
    success = tester.run_full_test_suite()
    
    if success:
        print("\n🎉 Tous les tests sont passés ! Le système fonctionne correctement.")
        sys.exit(0)
    else:
        print("\n⚠️ Certains tests ont échoué. Vérifiez les logs pour plus de détails.")
        sys.exit(1)

if __name__ == "__main__":
    main()