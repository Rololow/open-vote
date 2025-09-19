#!/usr/bin/env python3
"""
Mock Blockchain Server avec Notifications Temps Réel
Simule l'API blockchain pour tester l'interface web
Inclut WebSocket pour les notifications push
"""

from flask import Flask, jsonify, request
from flask_cors import CORS
from flask_socketio import SocketIO, emit, join_room, leave_room
from datetime import datetime, timezone, timedelta
import uuid
import json
import threading
import time

app = Flask(__name__)
CORS(app)  # Permet les requêtes cross-origin
socketio = SocketIO(app, cors_allowed_origins="*")  # WebSocket avec CORS

# Données de test
mock_laws = [
    {
        "id": str(uuid.uuid4()),
        "title": "Loi sur la Protection des Données Citoyennes",
        "content": "Cette loi vise à protéger les données personnelles des citoyens dans l'écosystème numérique gouvernemental.",
        "summary": "Protection renforcée des données personnelles des citoyens",
        "category": "Numérique",
        "status": "Active",
        "author": "0x1234567890abcdef",
        "created_at": "2024-01-15T10:30:00Z",
        "votes_for": 156,
        "votes_against": 23,
        "votes_abstain": 12
    },
    {
        "id": str(uuid.uuid4()),
        "title": "Règlement sur la Transparence Budgétaire",
        "content": "Établit les règles de transparence pour la publication des budgets publics sur la blockchain.",
        "summary": "Transparence obligatoire des budgets publics",
        "category": "Finances",
        "status": "Voting",
        "author": "0xabcdef1234567890",
        "created_at": "2024-02-20T14:45:00Z",
        "votes_for": 89,
        "votes_against": 45,
        "votes_abstain": 8
    },
    {
        "id": str(uuid.uuid4()),
        "title": "Décret sur la Participation Citoyenne Numérique",
        "content": "Définit les modalités de participation des citoyens aux décisions publiques via la plateforme blockchain.",
        "summary": "Cadre pour la participation citoyenne en ligne",
        "category": "Gouvernance",
        "status": "InReview",
        "author": "0x5678901234abcdef",
        "created_at": "2024-03-10T09:15:00Z",
        "votes_for": 0,
        "votes_against": 0,
        "votes_abstain": 0
    }
]

mock_accounts = [
    {
        "id": str(uuid.uuid4()),
        "public_key": "0x1234567890abcdef",
        "display_name": "Marie Dupont",
        "reputation": 85,
        "created_at": "2024-01-01T00:00:00Z"
    },
    {
        "id": str(uuid.uuid4()),
        "public_key": "0xabcdef1234567890", 
        "display_name": "Jean Martin",
        "reputation": 92,
        "created_at": "2024-01-15T00:00:00Z"
    }
]

mock_blocks = [
    {
        "number": 0,
        "hash": "0x" + "0" * 64,
        "previous_hash": "0x" + "0" * 64,
        "timestamp": "2024-01-01T00:00:00Z",
        "transaction_count": 1
    },
    {
        "number": 1,
        "hash": "0x" + "1" * 64,
        "previous_hash": "0x" + "0" * 64,
        "timestamp": "2024-01-15T10:30:00Z",
        "transaction_count": 3
    }
]

@app.route('/')
def root():
    return jsonify({
        "name": "Mock E-Government Blockchain API",
        "version": "0.1.0-mock",
        "description": "API simulée pour le système de blockchain e-gouvernement",
        "status": "running",
        "endpoints": {
            "health": "/health",
            "stats": "/stats", 
            "blockchain_info": "/info",
            "blocks": "/api/blocks",
            "transactions": "/api/transactions",
            "accounts": "/api/accounts",
            "laws": "/api/laws",
            "proposals": "/api/proposals",
            "votes": "/api/votes"
        }
    })

@app.route('/health')
def health():
    return jsonify({
        "status": "healthy",
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "service": "mock-blockchain-server"
    })

@app.route('/stats')
def stats():
    return jsonify({
        "total_blocks": len(mock_blocks),
        "total_laws": len(mock_laws),
        "total_accounts": len(mock_accounts),
        "active_laws": len([l for l in mock_laws if l["status"] == "Active"]),
        "pending_votes": len([l for l in mock_laws if l["status"] == "Voting"])
    })

@app.route('/info')
def blockchain_info():
    return jsonify({
        "network": "mock-egovern",
        "latest_block": max(mock_blocks, key=lambda x: x["number"]) if mock_blocks else None,
        "total_blocks": len(mock_blocks),
        "consensus": "mock-consensus"
    })

@app.route('/api/blocks')
def get_blocks():
    page = int(request.args.get('page', 1))
    limit = min(int(request.args.get('limit', 10)), 100)
    
    start_idx = (page - 1) * limit
    end_idx = start_idx + limit
    
    blocks = mock_blocks[start_idx:end_idx]
    
    return jsonify({
        "blocks": blocks,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": len(mock_blocks),
            "total_pages": (len(mock_blocks) + limit - 1) // limit
        }
    })

@app.route('/api/laws')
def get_laws():
    return jsonify({
        "laws": mock_laws,
        "total": len(mock_laws)
    })

@app.route('/api/accounts', methods=['GET', 'POST'])
def handle_accounts():
    if request.method == 'GET':
        return jsonify({
            "accounts": mock_accounts,
            "total": len(mock_accounts)
        })
    
    elif request.method == 'POST':
        data = request.get_json()
        new_account = {
            "id": str(uuid.uuid4()),
            "public_key": data.get("public_key", f"0x{uuid.uuid4().hex}"),
            "display_name": data.get("display_name", "Nouvel Utilisateur"),
            "reputation": 0,
            "created_at": datetime.now(timezone.utc).isoformat()
        }
        mock_accounts.append(new_account)
        
        return jsonify({
            "status": "success",
            "message": "Compte créé avec succès",
            "account": new_account
        })

@app.route('/api/votes', methods=['POST'])
def submit_vote():
    data = request.get_json()
    
    try:
        law_id = data.get("law_id")
        vote_type = data.get("vote_type")
        user_id = data.get("user_id", "demo-user")
        user_name = data.get("user_name", "Utilisateur Demo")
        
        if not law_id or not vote_type:
            log_failed_action(
                user_id=user_id,
                user_name=user_name,
                action_type='VOTE_FAILED',
                error_message='Paramètres de vote manquants',
                details={'law_id': law_id, 'vote_type': vote_type}
            )
            return jsonify({"error": "law_id et vote_type requis"}), 400
        
        vote_id = str(uuid.uuid4())
        
        # Enregistrer le vote dans l'audit
        log_audit_event(
            user_id=user_id,
            user_name=user_name,
            action_type='VOTE',
            action_description=f'Vote {vote_type} sur la loi {law_id}',
            details={
                'law_id': law_id,
                'vote_type': vote_type,
                'vote_id': vote_id,
                'vote_method': 'web_interface'
            }
        )
        
        # Notifier les administrateurs du nouveau vote
        vote_emoji = "✅" if vote_type == "for" else "❌" if vote_type == "against" else "⚪"
        broadcast_notification(
            'new_vote',
            f'Nouveau vote {vote_emoji}',
            f'{user_name} a voté "{vote_type}" sur une proposition',
            data={'user_id': user_id, 'law_id': law_id, 'vote_type': vote_type},
            target_role='administrator'
        )
        
        return jsonify({
            "status": "success",
            "message": "Vote enregistré avec succès",
            "vote_id": vote_id,
            "law_id": law_id,
            "vote_type": vote_type
        })
        
    except Exception as e:
        log_failed_action(
            user_id=data.get("user_id", "unknown"),
            user_name=data.get("user_name", "Unknown"),
            action_type='VOTE_ERROR',
            error_message=str(e),
            details=data
        )
        return jsonify({"error": f"Erreur lors du vote: {str(e)}"}), 500

@app.route('/api/laws', methods=['POST'])
def create_law():
    data = request.get_json()
    
    new_law = {
        "id": str(uuid.uuid4()),
        "title": data.get("title", "Nouvelle Loi"),
        "content": data.get("content", ""),
        "summary": data.get("summary", ""),
        "category": data.get("category", "Général"),
        "status": "Draft",
        "author": data.get("author_public_key", "0x" + uuid.uuid4().hex),
        "created_at": datetime.now(timezone.utc).isoformat(),
        "votes_for": 0,
        "votes_against": 0,
        "votes_abstain": 0
    }
    
    mock_laws.append(new_law)
    
    return jsonify({
        "status": "success",
        "message": "Loi créée avec succès",
        "law": new_law
    })

@app.route('/api/transactions')
def get_transactions():
    return jsonify({
        "transactions": [],
        "message": "Fonctionnalité en développement"
    })

@app.route('/api/laws/<law_id>', methods=['GET'])
def get_law_details(law_id):
    """Get detailed information about a specific law"""
    # Find the law by ID in our mock data
    law = None
    for mock_law in mock_laws:
        if mock_law["id"] == law_id:
            # Create detailed version with full content
            law = mock_law.copy()
            law.update({
                "content": f"Cette loi vise à {mock_law['summary'].lower()}.\n\nArticle 1 - Principes généraux:\nTexte détaillé de l'article 1 de la loi '{mock_law['title']}'.\n\nArticle 2 - Modalités d'application:\nDétails des modalités d'application et de mise en œuvre.\n\nArticle 3 - Dispositions finales:\nClauses de révision et d'entrée en vigueur.",
                "version": "1.0",
                "amendments": [],
                "full_text": True
            })
            break
    
    if not law:
        return jsonify({"error": "Loi non trouvée"}), 404
    
    return jsonify({"law": law})

@app.route('/api/vote', methods=['POST'])
def submit_vote_v2():
    """Enhanced vote submission endpoint"""
    try:
        data = request.get_json()
        
        # Validation des données
        required_fields = ['law_id', 'voter_id', 'vote_type']
        if not all(field in data for field in required_fields):
            return jsonify({"error": "Missing required fields: law_id, voter_id, vote_type"}), 400
        
        if data['vote_type'] not in ['for', 'against', 'abstain']:
            return jsonify({"error": "Invalid vote_type. Must be 'for', 'against', or 'abstain'"}), 400
        
        # Simulation d'enregistrement du vote
        vote_record = {
            "id": str(uuid.uuid4()),
            "law_id": data['law_id'],
            "voter_id": data['voter_id'],
            "vote_type": data['vote_type'],
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "weight": data.get('weight', 50),
            "status": "recorded"
        }
        
        # Envoyer une notification pour le nouveau vote
        law_title = f"Loi #{data['law_id']}"  # En réalité, on récupérerait le vrai titre
        vote_emoji = "👍" if data['vote_type'] == 'for' else "👎" if data['vote_type'] == 'against' else "🤐"
        
        broadcast_notification(
            'vote',
            f'Nouveau vote enregistré {vote_emoji}',
            f'Un vote "{data["vote_type"]}" a été enregistré pour {law_title}',
            data={
                'law_id': data['law_id'],
                'vote_type': data['vote_type'],
                'vote_id': vote_record['id']
            },
            target_role='administrator'  # Notifier les administrateurs
        )

        return jsonify({
            "message": "Vote enregistré avec succès",
            "vote": vote_record,
            "success": True
        }), 201
        
    except Exception as e:
        return jsonify({"error": f"Erreur lors de l'enregistrement du vote: {str(e)}"}), 500

@app.route('/api/auth/login', methods=['POST'])
def login():
    """User authentication endpoint"""
    try:
        data = request.get_json()
        
        email = data.get('email')
        password = data.get('password')
        
        if not email or not password:
            return jsonify({"error": "Email et mot de passe requis"}), 400
        
        # Comptes de démonstration
        demo_users = {
            "marie@example.com": {
                "id": "703837ea-1b7e-4601-b3d7-f6cc065306b8",
                "name": "Marie Dupont",
                "email": "marie@example.com",
                "role": "citizen",
                "reputation": 85,
                "password": "demo123"
            },
            "jean@example.com": {
                "id": "9c08013d-73f4-41b8-b875-64e5037b342e",
                "name": "Jean Martin",
                "email": "jean@example.com",
                "role": "representative",
                "reputation": 92,
                "password": "demo123"
            },
            "admin@example.com": {
                "id": "admin-12345",
                "name": "Administrateur",
                "email": "admin@example.com",
                "role": "administrator",
                "reputation": 100,
                "password": "admin123"
            }
        }
        
        user = demo_users.get(email)
        if not user or user['password'] != password:
            # Enregistrer la tentative de connexion échouée
            log_failed_action(
                user_id=None,
                user_name=email,
                action_type='LOGIN_FAILED',
                error_message='Email ou mot de passe incorrect',
                details={'email': email, 'ip': request.environ.get('REMOTE_ADDR', 'unknown')}
            )
            return jsonify({"error": "Email ou mot de passe incorrect"}), 401
        
        # Créer un token simple (dans une vraie app, utiliser JWT)
        token = f"token_{user['id']}_{int(datetime.now().timestamp())}"
        
        # Enregistrer la connexion réussie dans l'audit
        log_audit_event(
            user_id=user['id'],
            user_name=user['name'],
            action_type='LOGIN',
            action_description=f'Connexion réussie pour {user["role"]}',
            details={
                'email': email,
                'role': user['role'],
                'token_created': token[:20] + '...',
                'user_agent': request.environ.get('HTTP_USER_AGENT', 'unknown')[:100]
            }
        )
        
        # Retourner les informations utilisateur
        user_data = {
            "id": user['id'],
            "name": user['name'],
            "email": user['email'],
            "role": user['role'],
            "reputation": user['reputation'],
            "token": token,
            "login_time": datetime.now(timezone.utc).isoformat()
        }
        
        # Envoyer une notification de connexion aux administrateurs
        role_emoji = "👤" if user['role'] == 'citizen' else "🏛️" if user['role'] == 'representative' else "👑"
        broadcast_notification(
            'user_login',
            f'Nouvelle connexion {role_emoji}',
            f'{user["name"]} ({user["role"]}) s\'est connecté(e)',
            data={'user_id': user['id'], 'user_name': user['name'], 'role': user['role']},
            target_role='administrator'
        )

        return jsonify({
            "message": "Connexion réussie",
            "user": user_data,
            "success": True
        }), 200
        
    except Exception as e:
        return jsonify({"error": f"Erreur de connexion: {str(e)}"}), 500

@app.route('/api/auth/register', methods=['POST'])
def register():
    """User registration endpoint"""
    try:
        data = request.get_json()
        
        required_fields = ['name', 'email', 'password', 'role']
        if not all(field in data for field in required_fields):
            return jsonify({"error": "Tous les champs sont requis"}), 400
        
        name = data['name']
        email = data['email']
        password = data['password']
        role = data['role']
        
        if len(password) < 6:
            return jsonify({"error": "Le mot de passe doit contenir au moins 6 caractères"}), 400
        
        if role not in ['citizen', 'representative', 'administrator']:
            return jsonify({"error": "Rôle invalide"}), 400
        
        # Vérifier si l'email existe déjà (simulation)
        existing_emails = ["marie@example.com", "jean@example.com", "admin@example.com"]
        if email in existing_emails:
            # Enregistrer la tentative d'inscription échouée
            log_failed_action(
                user_id=None,
                user_name=name,
                action_type='REGISTER_FAILED', 
                error_message='Email déjà utilisé',
                details={'email': email, 'attempted_role': role}
            )
            return jsonify({"error": "Cet email est déjà utilisé"}), 409
        
        # Créer un nouvel utilisateur
        user_id = str(uuid.uuid4())
        token = f"token_{user_id}_{int(datetime.now().timestamp())}"
        
        # Enregistrer la création de compte dans l'audit
        log_audit_event(
            user_id=user_id,
            user_name=name,
            action_type='REGISTER',
            action_description=f'Nouveau compte créé avec le rôle {role}',
            details={
                'email': email,
                'role': role,
                'initial_reputation': 50,
                'registration_method': 'web_form',
                'user_agent': request.environ.get('HTTP_USER_AGENT', 'unknown')[:100]
            }
        )
        
        user_data = {
            "id": user_id,
            "name": name,
            "email": email,
            "role": role,
            "reputation": 50,  # Réputation de base
            "token": token,
            "registration_time": datetime.now(timezone.utc).isoformat()
        }
        
        # Envoyer une notification de bienvenue à l'utilisateur
        broadcast_notification(
            'welcome',
            '🎉 Bienvenue sur la plateforme !',
            f'Bonjour {name} ! Votre compte a été créé avec succès. Vous pouvez maintenant participer aux votes.',
            target_user=user_id
        )
        
        # Notifier les administrateurs du nouveau compte
        role_emoji = "👤" if role == 'citizen' else "🏛️" if role == 'representative' else "👑"
        broadcast_notification(
            'new_user',
            f'Nouveau compte créé {role_emoji}',
            f'{name} a créé un compte avec le rôle "{role}"',
            data={'user_id': user_id, 'user_name': name, 'role': role},
            target_role='administrator'
        )

        return jsonify({
            "message": "Compte créé avec succès",
            "user": user_data,
            "success": True
        }), 201
        
    except Exception as e:
        return jsonify({"error": f"Erreur lors de la création du compte: {str(e)}"}), 500

@app.route('/api/auth/profile', methods=['GET'])
def get_profile():
    """Get user profile information"""
    # Simulation de récupération du profil
    # Dans une vraie app, on vérifierait le token d'authentification
    
    return jsonify({
        "user": {
            "id": "demo-user",
            "name": "Utilisateur Demo",
            "email": "demo@example.com",
            "role": "citizen",
            "reputation": 75,
            "votes_cast": 15,
            "laws_proposed": 2,
            "join_date": "2024-01-01T00:00:00Z"
        }
    })

# =============================================================================
# SYSTÈME DE NOTIFICATIONS TEMPS RÉEL (WebSocket)
# =============================================================================

# Stockage des utilisateurs connectés et notifications
connected_users = {}
notification_queue = []

# Système d'audit et historique
audit_logs = []

# Variable globale pour SocketIO
socketio_instance = None

@socketio.on('connect')
def on_connect():
    """Gestion de la connexion WebSocket"""
    print(f"👤 Nouvelle connexion WebSocket: {request.sid}")
    emit('connected', {'message': 'Connexion établie au système de notifications'})

@socketio.on('disconnect')
def on_disconnect():
    """Gestion de la déconnexion WebSocket"""
    if request.sid in connected_users:
        user = connected_users[request.sid]
        print(f"👋 Déconnexion de {user.get('name', 'Anonyme')}")
        del connected_users[request.sid]
    print(f"❌ Déconnexion WebSocket: {request.sid}")

@socketio.on('authenticate')
def on_authenticate(data):
    """Authentification de l'utilisateur WebSocket"""
    try:
        token = data.get('token')
        user_info = data.get('user')
        
        if token and user_info:
            connected_users[request.sid] = {
                'user_id': user_info.get('id'),
                'name': user_info.get('name'),
                'role': user_info.get('role'),
                'token': token,
                'connected_at': datetime.now(timezone.utc).isoformat()
            }
            
            # Rejoindre la room selon le rôle
            join_room(f"role_{user_info.get('role', 'citizen')}")
            join_room(f"user_{user_info.get('id')}")
            
            print(f"🔐 Authentification WebSocket réussie: {user_info.get('name')} ({user_info.get('role')})")
            
            emit('authenticated', {
                'success': True,
                'message': f'Bienvenue {user_info.get("name")} ! Notifications activées.'
            })
            
            # Envoyer les notifications en attente
            send_pending_notifications(user_info.get('id'))
            
        else:
            emit('authentication_error', {'error': 'Token ou informations utilisateur manquants'})
            
    except Exception as e:
        print(f"❌ Erreur d'authentification WebSocket: {e}")
        emit('authentication_error', {'error': 'Erreur d\'authentification'})

def send_pending_notifications(user_id):
    """Envoie les notifications en attente pour un utilisateur"""
    global notification_queue, socketio
    user_notifications = [n for n in notification_queue if n.get('target_user') == user_id or n.get('target_user') == 'all']
    
    for notification in user_notifications:
        try:
            socketio.emit('notification', notification, room=f"user_{user_id}")
        except Exception as e:
            print(f"⚠️  Erreur lors de l'envoi de notification en attente: {e}")

def broadcast_notification(notification_type, title, message, data=None, target_role=None, target_user=None):
    """Diffuse une notification à tous les utilisateurs connectés ou à un groupe spécifique"""
    global notification_queue, socketio
    
    notification = {
        'id': str(uuid.uuid4()),
        'type': notification_type,
        'title': title,
        'message': message,
        'data': data or {},
        'timestamp': datetime.now(timezone.utc).isoformat(),
        'target_role': target_role,
        'target_user': target_user
    }
    
    # Ajouter à la queue pour les utilisateurs déconnectés
    notification_queue.append(notification)
    
    # Limiter la taille de la queue
    if len(notification_queue) > 100:
        notification_queue = notification_queue[-50:]
    
    # Diffuser selon la cible si socketio est disponible
    try:
        if target_user:
            socketio.emit('notification', notification, room=f"user_{target_user}")
            print(f"📨 Notification envoyée à l'utilisateur {target_user}: {title}")
        elif target_role:
            socketio.emit('notification', notification, room=f"role_{target_role}")
            print(f"📨 Notification envoyée au rôle {target_role}: {title}")
        else:
            socketio.emit('notification', notification)
            print(f"📢 Notification diffusée à tous: {title}")
    except Exception as e:
        print(f"⚠️  Erreur lors de l'envoi de notification WebSocket: {e}")

# Endpoint pour tester les notifications
@app.route('/api/notifications/test', methods=['POST'])
def test_notification():
    """Endpoint pour tester le système de notifications"""
    global notification_queue
    try:
        data = request.get_json() or {}
        
        notification_type = data.get('type', 'info')
        title = data.get('title', 'Notification de test')
        message = data.get('message', 'Ceci est un message de test')
        target_role = data.get('target_role')
        target_user = data.get('target_user')
        
        broadcast_notification(notification_type, title, message, target_role=target_role, target_user=target_user)
        
        return jsonify({
            'success': True,
            'message': 'Notification de test envoyée',
            'notification_count': len(notification_queue)
        })
        
    except Exception as e:
        print(f"❌ Erreur dans test_notification: {e}")
        return jsonify({'error': str(e)}), 500

# Endpoint pour obtenir les notifications d'un utilisateur
@app.route('/api/notifications/<user_id>', methods=['GET'])
def get_user_notifications(user_id):
    """Récupère les notifications d'un utilisateur"""
    global notification_queue
    try:
        user_notifications = [
            n for n in notification_queue 
            if n.get('target_user') == user_id or n.get('target_user') == 'all'
        ]
        
        # Retourner les 20 dernières notifications
        recent_notifications = sorted(user_notifications, key=lambda x: x['timestamp'], reverse=True)[:20]
        
        return jsonify({
            'notifications': recent_notifications,
            'count': len(recent_notifications)
        })
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

# =============================================================================
# SYSTÈME D'HISTORIQUE ET AUDIT
# =============================================================================

def log_audit_event(user_id, user_name, action_type, action_description, details=None, ip_address=None):
    """Enregistre un événement d'audit"""
    global audit_logs
    
    audit_entry = {
        'id': str(uuid.uuid4()),
        'user_id': user_id,
        'user_name': user_name,
        'action_type': action_type,  # LOGIN, LOGOUT, VOTE, CREATE_ACCOUNT, etc.
        'action_description': action_description,
        'details': details or {},
        'ip_address': ip_address or request.environ.get('REMOTE_ADDR', 'unknown'),
        'user_agent': request.environ.get('HTTP_USER_AGENT', 'unknown'),
        'timestamp': datetime.now(timezone.utc).isoformat(),
        'session_id': request.environ.get('HTTP_X_SESSION_ID', 'unknown'),
        'success': True
    }
    
    audit_logs.append(audit_entry)
    
    # Limiter la taille des logs (garder les 1000 derniers)
    if len(audit_logs) > 1000:
        audit_logs = audit_logs[-500:]
    
    print(f"📋 Audit: {user_name} - {action_type} - {action_description}")
    return audit_entry

def log_failed_action(user_id, user_name, action_type, error_message, details=None):
    """Enregistre une action échouée"""
    global audit_logs
    
    audit_entry = {
        'id': str(uuid.uuid4()),
        'user_id': user_id or 'anonymous',
        'user_name': user_name or 'Anonyme',
        'action_type': action_type,
        'action_description': f'Échec: {error_message}',
        'details': details or {},
        'ip_address': request.environ.get('REMOTE_ADDR', 'unknown'),
        'user_agent': request.environ.get('HTTP_USER_AGENT', 'unknown'),
        'timestamp': datetime.now(timezone.utc).isoformat(),
        'session_id': request.environ.get('HTTP_X_SESSION_ID', 'unknown'),
        'success': False,
        'error': error_message
    }
    
    audit_logs.append(audit_entry)
    
    # Limiter la taille des logs
    if len(audit_logs) > 1000:
        audit_logs = audit_logs[-500:]
    
    print(f"❌ Audit Échec: {user_name} - {action_type} - {error_message}")
    return audit_entry

# Endpoints pour l'historique et l'audit
@app.route('/api/audit/logs', methods=['GET'])
def get_audit_logs():
    """Récupère les logs d'audit (admin uniquement)"""
    global audit_logs
    try:
        # Dans une vraie app, vérifier les permissions admin ici
        
        # Paramètres de pagination
        page = int(request.args.get('page', 1))
        per_page = int(request.args.get('per_page', 50))
        user_id = request.args.get('user_id')
        action_type = request.args.get('action_type')
        start_date = request.args.get('start_date')
        end_date = request.args.get('end_date')
        
        # Filtrer les logs
        filtered_logs = audit_logs.copy()
        
        if user_id:
            filtered_logs = [log for log in filtered_logs if log['user_id'] == user_id]
        
        if action_type:
            filtered_logs = [log for log in filtered_logs if log['action_type'] == action_type]
        
        if start_date:
            filtered_logs = [log for log in filtered_logs if log['timestamp'] >= start_date]
        
        if end_date:
            filtered_logs = [log for log in filtered_logs if log['timestamp'] <= end_date]
        
        # Trier par date (plus récent en premier)
        filtered_logs.sort(key=lambda x: x['timestamp'], reverse=True)
        
        # Pagination
        total = len(filtered_logs)
        start_idx = (page - 1) * per_page
        end_idx = start_idx + per_page
        paginated_logs = filtered_logs[start_idx:end_idx]
        
        return jsonify({
            'logs': paginated_logs,
            'pagination': {
                'page': page,
                'per_page': per_page,
                'total': total,
                'pages': (total + per_page - 1) // per_page
            },
            'filters': {
                'user_id': user_id,
                'action_type': action_type,
                'start_date': start_date,
                'end_date': end_date
            }
        })
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/audit/stats', methods=['GET'])
def get_audit_stats():
    """Récupère les statistiques d'audit"""
    global audit_logs
    try:
        # Compter par type d'action
        action_counts = {}
        user_counts = {}
        success_count = 0
        failed_count = 0
        
        for log in audit_logs:
            # Compter par type d'action
            action_type = log['action_type']
            action_counts[action_type] = action_counts.get(action_type, 0) + 1
            
            # Compter par utilisateur
            user_name = log['user_name']
            user_counts[user_name] = user_counts.get(user_name, 0) + 1
            
            # Compter succès/échecs
            if log['success']:
                success_count += 1
            else:
                failed_count += 1
        
        # Top utilisateurs les plus actifs
        top_users = sorted(user_counts.items(), key=lambda x: x[1], reverse=True)[:10]
        
        # Actions récentes (dernières 24h)
        now = datetime.now(timezone.utc)
        yesterday = now - timedelta(days=1)
        recent_logs = [
            log for log in audit_logs 
            if datetime.fromisoformat(log['timestamp'].replace('Z', '+00:00')) > yesterday
        ]
        
        return jsonify({
            'total_logs': len(audit_logs),
            'success_rate': round((success_count / len(audit_logs)) * 100, 2) if audit_logs else 0,
            'action_counts': action_counts,
            'top_users': top_users,
            'recent_activity': len(recent_logs),
            'failed_actions': failed_count,
            'success_actions': success_count
        })
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/audit/user/<user_id>', methods=['GET'])
def get_user_audit_history(user_id):
    """Récupère l'historique d'un utilisateur spécifique"""
    global audit_logs
    try:
        user_logs = [log for log in audit_logs if log['user_id'] == user_id]
        user_logs.sort(key=lambda x: x['timestamp'], reverse=True)
        
        # Limiter aux 100 dernières actions
        user_logs = user_logs[:100]
        
        # Statistiques utilisateur
        action_counts = {}
        for log in user_logs:
            action_type = log['action_type']
            action_counts[action_type] = action_counts.get(action_type, 0) + 1
        
        return jsonify({
            'user_id': user_id,
            'logs': user_logs,
            'total_actions': len(user_logs),
            'action_breakdown': action_counts,
            'last_activity': user_logs[0]['timestamp'] if user_logs else None
        })
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/audit/export', methods=['GET'])
def export_audit_logs():
    """Exporte les logs d'audit en CSV (admin uniquement)"""
    global audit_logs
    try:
        import io
        import csv
        
        output = io.StringIO()
        writer = csv.writer(output)
        
        # En-têtes CSV
        writer.writerow([
            'ID', 'Timestamp', 'User ID', 'User Name', 'Action Type', 
            'Description', 'Success', 'IP Address', 'User Agent'
        ])
        
        # Données
        for log in sorted(audit_logs, key=lambda x: x['timestamp'], reverse=True):
            writer.writerow([
                log['id'],
                log['timestamp'],
                log['user_id'],
                log['user_name'],
                log['action_type'],
                log['action_description'],
                log['success'],
                log['ip_address'],
                log['user_agent']
            ])
        
        csv_data = output.getvalue()
        output.close()
        
        return jsonify({
            'csv_data': csv_data,
            'filename': f'audit_logs_{datetime.now().strftime("%Y%m%d_%H%M%S")}.csv',
            'total_records': len(audit_logs)
        })
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

# =============================================================================
# SYSTÈME D'ANALYTICS ET VISUALISATION
# =============================================================================

@app.route('/api/analytics/overview', methods=['GET'])
def get_analytics_overview():
    """Récupère les données de vue d'ensemble pour le dashboard analytics"""
    try:
        # Simulations de données avancées basées sur l'audit
        now = datetime.now(timezone.utc)
        
        # Données de vote par période
        vote_trends = [
            {'date': (now - timedelta(days=6)).strftime('%Y-%m-%d'), 'votes': 45, 'for': 32, 'against': 10, 'abstain': 3},
            {'date': (now - timedelta(days=5)).strftime('%Y-%m-%d'), 'votes': 38, 'for': 25, 'against': 8, 'abstain': 5},
            {'date': (now - timedelta(days=4)).strftime('%Y-%m-%d'), 'votes': 52, 'for': 41, 'against': 7, 'abstain': 4},
            {'date': (now - timedelta(days=3)).strftime('%Y-%m-%d'), 'votes': 41, 'for': 28, 'against': 9, 'abstain': 4},
            {'date': (now - timedelta(days=2)).strftime('%Y-%m-%d'), 'votes': 47, 'for': 35, 'against': 8, 'abstain': 4},
            {'date': (now - timedelta(days=1)).strftime('%Y-%m-%d'), 'votes': 55, 'for': 42, 'against': 9, 'abstain': 4},
            {'date': now.strftime('%Y-%m-%d'), 'votes': len([log for log in audit_logs if log['action_type'] == 'VOTE']), 'for': 1, 'against': 0, 'abstain': 0}
        ]
        
        # Répartition des utilisateurs par rôle
        user_distribution = {
            'citizens': 156,
            'representatives': 23,
            'administrators': 5
        }
        
        # Activité par heure (dernières 24h)
        hourly_activity = []
        for hour in range(24):
            activity_time = now.replace(hour=hour, minute=0, second=0, microsecond=0)
            # Simulation basée sur l'heure
            base_activity = max(1, int(20 * (1 + 0.8 * abs(12 - hour) / 12)))  # Plus d'activité aux heures de pointe
            hourly_activity.append({
                'hour': hour,
                'time': activity_time.strftime('%H:00'),
                'connections': base_activity + (3 if hour >= 8 and hour <= 20 else 1),
                'votes': max(0, base_activity - 5 + (2 if hour >= 10 and hour <= 18 else 0)),
                'actions': base_activity + 2
            })
        
        # Top lois par engagement
        top_laws = [
            {'id': 'law-001', 'title': 'Protection de l\'Environnement', 'total_votes': 234, 'engagement_score': 92},
            {'id': 'law-002', 'title': 'Réforme Éducative', 'total_votes': 198, 'engagement_score': 87},
            {'id': 'law-003', 'title': 'Infrastructure Numérique', 'total_votes': 156, 'engagement_score': 81},
            {'id': 'law-004', 'title': 'Santé Publique', 'total_votes': 189, 'engagement_score': 79},
            {'id': 'law-005', 'title': 'Sécurité Sociale', 'total_votes': 167, 'engagement_score': 76}
        ]
        
        # Métriques de performance système
        system_metrics = {
            'response_time_avg': 45.6,  # ms
            'uptime_percentage': 99.8,
            'active_sessions': 23,
            'total_transactions': 1247,
            'success_rate': 98.2
        }
        
        return jsonify({
            'vote_trends': vote_trends,
            'user_distribution': user_distribution,
            'hourly_activity': hourly_activity,
            'top_laws': top_laws,
            'system_metrics': system_metrics,
            'generated_at': now.isoformat()
        })
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/analytics/votes', methods=['GET'])
def get_vote_analytics():
    """Analytics détaillées des votes"""
    try:
        # Analyse des patterns de vote
        vote_patterns = {
            'by_time_of_day': [
                {'hour': h, 'count': max(1, int(15 * (1 + 0.5 * abs(14 - h) / 14)))} 
                for h in range(24)
            ],
            'by_day_of_week': [
                {'day': 'Lundi', 'count': 45},
                {'day': 'Mardi', 'count': 52},
                {'day': 'Mercredi', 'count': 48},
                {'day': 'Jeudi', 'count': 41},
                {'day': 'Vendredi', 'count': 38},
                {'day': 'Samedi', 'count': 22},
                {'day': 'Dimanche', 'count': 18}
            ],
            'participation_rate': {
                'total_eligible': 184,
                'total_voted': 156,
                'rate': 84.8
            }
        }
        
        # Analyse démographique
        demographic_analysis = {
            'by_role': [
                {'role': 'Citoyens', 'for': 78, 'against': 12, 'abstain': 8},
                {'role': 'Représentants', 'for': 15, 'against': 6, 'abstain': 2},
                {'role': 'Administrateurs', 'for': 4, 'against': 1, 'abstain': 0}
            ],
            'by_age_group': [
                {'group': '18-25', 'for': 23, 'against': 4, 'abstain': 2},
                {'group': '26-35', 'for': 34, 'against': 7, 'abstain': 3},
                {'group': '36-50', 'for': 28, 'against': 5, 'abstain': 3},
                {'group': '51+', 'for': 12, 'against': 3, 'abstain': 2}
            ]
        }
        
        # Tendances temporelles
        temporal_trends = {
            'monthly': [
                {'month': 'Jan', 'votes': 234, 'laws': 12},
                {'month': 'Fév', 'votes': 198, 'laws': 9},
                {'month': 'Mar', 'votes': 267, 'laws': 15},
                {'month': 'Avr', 'votes': 189, 'laws': 11},
                {'month': 'Mai', 'votes': 223, 'laws': 13},
                {'month': 'Jun', 'votes': 245, 'laws': 14},
                {'month': 'Jul', 'votes': 178, 'laws': 8},
                {'month': 'Aoû', 'votes': 156, 'laws': 7},
                {'month': 'Sep', 'votes': 201, 'laws': 10}
            ]
        }
        
        return jsonify({
            'vote_patterns': vote_patterns,
            'demographic_analysis': demographic_analysis,
            'temporal_trends': temporal_trends
        })
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/analytics/engagement', methods=['GET'])
def get_engagement_analytics():
    """Analytics d'engagement des utilisateurs"""
    try:
        # Scores d'engagement par utilisateur (top 10)
        top_engaged_users = [
            {'name': 'Marie Dupont', 'score': 95, 'votes': 23, 'proposals': 3, 'comments': 45},
            {'name': 'Jean Martin', 'score': 92, 'votes': 21, 'proposals': 5, 'comments': 38},
            {'name': 'Sophie Bernard', 'score': 88, 'votes': 19, 'proposals': 2, 'comments': 41},
            {'name': 'Pierre Dubois', 'score': 85, 'votes': 18, 'proposals': 4, 'comments': 32},
            {'name': 'Anne Moreau', 'score': 82, 'votes': 17, 'proposals': 1, 'comments': 39},
            {'name': 'Michel Laurent', 'score': 79, 'votes': 16, 'proposals': 3, 'comments': 28},
            {'name': 'Claire Simon', 'score': 76, 'votes': 15, 'proposals': 2, 'comments': 33},
            {'name': 'François Petit', 'score': 73, 'votes': 14, 'proposals': 1, 'comments': 27},
            {'name': 'Isabelle Roux', 'score': 71, 'votes': 13, 'proposals': 2, 'comments': 31},
            {'name': 'Thomas Blanc', 'score': 68, 'votes': 12, 'proposals': 1, 'comments': 25}
        ]
        
        # Métriques d'engagement global
        engagement_metrics = {
            'average_session_duration': 342,  # secondes
            'pages_per_session': 4.7,
            'bounce_rate': 23.4,  # pourcentage
            'return_user_rate': 76.8,
            'active_users_7d': 89,
            'active_users_30d': 156
        }
        
        # Evolution de l'engagement
        engagement_evolution = [
            {'week': 'S1', 'new_users': 12, 'active_users': 67, 'engagement_score': 72},
            {'week': 'S2', 'new_users': 8, 'active_users': 71, 'engagement_score': 74},
            {'week': 'S3', 'new_users': 15, 'active_users': 78, 'engagement_score': 76},
            {'week': 'S4', 'new_users': 11, 'active_users': 82, 'engagement_score': 79},
            {'week': 'S5', 'new_users': 9, 'active_users': 85, 'engagement_score': 81},
            {'week': 'S6', 'new_users': 13, 'active_users': 89, 'engagement_score': 83}
        ]
        
        return jsonify({
            'top_engaged_users': top_engaged_users,
            'engagement_metrics': engagement_metrics,
            'engagement_evolution': engagement_evolution
        })
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

# =============================================================================
# SYSTÈME DE PROPOSITIONS CITOYENNES
# =============================================================================

# Base de données des propositions (simulation)
citizen_proposals = [
    {
        'id': 'prop-001',
        'title': 'Transport Public Gratuit',
        'description': 'Proposition pour rendre tous les transports publics gratuits dans les zones urbaines',
        'full_text': '''
## Proposition : Transport Public Gratuit

### Objectif
Rendre tous les transports publics (bus, métro, tramway) gratuits pour tous les citoyens dans les zones urbaines.

### Justification
- Réduction de la pollution atmosphérique
- Amélioration de l'accès à la mobilité pour tous
- Réduction des embouteillages
- Stimulation de l'économie locale

### Budget Estimé
- Coût annuel : 2.5 milliards €
- Financement par : taxe carbone et économies sur infrastructure routière

### Calendrier
- Phase pilote : 6 mois dans 3 villes
- Déploiement national : 2 ans
        ''',
        'author_id': '703837ea-1b7e-4601-b3d7-f6cc065306b8',
        'author_name': 'Marie Dupont',
        'category': 'Transport',
        'status': 'En révision',
        'created_at': (datetime.now(timezone.utc) - timedelta(days=5)).isoformat(),
        'updated_at': (datetime.now(timezone.utc) - timedelta(days=2)).isoformat(),
        'supporters': 1247,
        'signatures_required': 5000,
        'comments_count': 89,
        'upvotes': 1156,
        'downvotes': 91,
        'tags': ['transport', 'environnement', 'social'],
        'visibility': 'public',
        'estimated_budget': 2500000000,
        'implementation_timeline': '24 mois'
    },
    {
        'id': 'prop-002',
        'title': 'Énergie Solaire Obligatoire',
        'description': 'Obligation d\'installer des panneaux solaires sur tous les nouveaux bâtiments',
        'full_text': '''
## Proposition : Énergie Solaire Obligatoire

### Objectif
Rendre obligatoire l'installation de panneaux solaires sur tous les nouveaux bâtiments résidentiels et commerciaux.

### Justification
- Transition énergétique accélérée
- Réduction de la dépendance énergétique
- Création d'emplois verts
- Baisse des factures énergétiques à long terme

### Mesures Complémentaires
- Subventions pour l'installation
- Formation professionnelle spécialisée
- Réglementation technique mise à jour
        ''',
        'author_id': '9c08013d-73f4-41b8-b875-64e5037b342e',
        'author_name': 'Jean Martin',
        'category': 'Environnement',
        'status': 'Collecte signatures',
        'created_at': (datetime.now(timezone.utc) - timedelta(days=12)).isoformat(),
        'updated_at': (datetime.now(timezone.utc) - timedelta(days=1)).isoformat(),
        'supporters': 3456,
        'signatures_required': 5000,
        'comments_count': 156,
        'upvotes': 3201,
        'downvotes': 255,
        'tags': ['environnement', 'énergie', 'réglementation'],
        'visibility': 'public',
        'estimated_budget': 850000000,
        'implementation_timeline': '18 mois'
    },
    {
        'id': 'prop-003',
        'title': 'Semaine de 4 Jours',
        'description': 'Expérimentation de la semaine de travail de 4 jours dans le secteur public',
        'full_text': '''
## Proposition : Semaine de Travail de 4 Jours

### Objectif
Lancer une expérimentation de la semaine de travail de 4 jours dans le secteur public.

### Bénéfices Attendus
- Amélioration de la qualité de vie
- Réduction du stress et burn-out
- Maintien de la productivité
- Économies énergétiques

### Phase d'Expérimentation
- Durée : 12 mois
- Secteurs pilotes : administration, éducation
- Évaluation trimestrielle des résultats
        ''',
        'author_id': 'user-789',
        'author_name': 'Sophie Lemoine',
        'category': 'Travail',
        'status': 'Brouillon',
        'created_at': (datetime.now(timezone.utc) - timedelta(days=3)).isoformat(),
        'updated_at': (datetime.now(timezone.utc) - timedelta(hours=6)).isoformat(),
        'supporters': 892,
        'signatures_required': 5000,
        'comments_count': 34,
        'upvotes': 823,
        'downvotes': 69,
        'tags': ['travail', 'bien-être', 'expérimentation'],
        'visibility': 'public',
        'estimated_budget': 125000000,
        'implementation_timeline': '12 mois'
    }
]

@app.route('/api/proposals', methods=['GET'])
def get_proposals():
    """Récupère la liste des propositions citoyennes"""
    try:
        # Paramètres de filtrage et pagination
        status_filter = request.args.get('status')
        category_filter = request.args.get('category')
        author_filter = request.args.get('author')
        search_query = request.args.get('search', '').lower()
        sort_by = request.args.get('sort_by', 'created_at')  # created_at, supporters, upvotes
        sort_order = request.args.get('sort_order', 'desc')
        page = int(request.args.get('page', 1))
        per_page = int(request.args.get('per_page', 10))
        
        # Filtrer les propositions
        filtered_proposals = citizen_proposals.copy()
        
        if status_filter:
            filtered_proposals = [p for p in filtered_proposals if p['status'] == status_filter]
        
        if category_filter:
            filtered_proposals = [p for p in filtered_proposals if p['category'] == category_filter]
        
        if author_filter:
            filtered_proposals = [p for p in filtered_proposals if p['author_id'] == author_filter]
        
        if search_query:
            filtered_proposals = [
                p for p in filtered_proposals 
                if search_query in p['title'].lower() or search_query in p['description'].lower()
            ]
        
        # Trier les propositions
        reverse = sort_order == 'desc'
        if sort_by == 'supporters':
            filtered_proposals.sort(key=lambda x: x['supporters'], reverse=reverse)
        elif sort_by == 'upvotes':
            filtered_proposals.sort(key=lambda x: x['upvotes'], reverse=reverse)
        else:  # created_at par défaut
            filtered_proposals.sort(key=lambda x: x['created_at'], reverse=reverse)
        
        # Pagination
        total = len(filtered_proposals)
        start_idx = (page - 1) * per_page
        end_idx = start_idx + per_page
        paginated_proposals = filtered_proposals[start_idx:end_idx]
        
        # Statistiques globales
        stats = {
            'total_proposals': len(citizen_proposals),
            'active_proposals': len([p for p in citizen_proposals if p['status'] in ['Collecte signatures', 'En révision']]),
            'total_supporters': sum(p['supporters'] for p in citizen_proposals),
            'categories': list(set(p['category'] for p in citizen_proposals))
        }
        
        return jsonify({
            'proposals': paginated_proposals,
            'pagination': {
                'page': page,
                'per_page': per_page,
                'total': total,
                'pages': (total + per_page - 1) // per_page
            },
            'stats': stats,
            'filters': {
                'status': status_filter,
                'category': category_filter,
                'author': author_filter,
                'search': search_query,
                'sort_by': sort_by,
                'sort_order': sort_order
            }
        })
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/proposals/<proposal_id>', methods=['GET'])
def get_proposal_details(proposal_id):
    """Récupère les détails d'une proposition spécifique"""
    try:
        proposal = next((p for p in citizen_proposals if p['id'] == proposal_id), None)
        
        if not proposal:
            return jsonify({'error': 'Proposition non trouvée'}), 404
        
        # Ajouter des commentaires simulés
        proposal_with_comments = proposal.copy()
        proposal_with_comments['comments'] = [
            {
                'id': 'comment-1',
                'author': 'Pierre Durand',
                'content': 'Excellente idée ! Cela pourrait vraiment changer notre quotidien.',
                'created_at': (datetime.now(timezone.utc) - timedelta(hours=12)).isoformat(),
                'upvotes': 23,
                'downvotes': 2
            },
            {
                'id': 'comment-2',
                'author': 'Anne Moreau',
                'content': 'Je soutiens cette proposition, mais il faudrait étudier l\'impact budgétaire.',
                'created_at': (datetime.now(timezone.utc) - timedelta(hours=8)).isoformat(),
                'upvotes': 18,
                'downvotes': 5
            },
            {
                'id': 'comment-3',
                'author': 'Michel Bernard',
                'content': 'Très intéressant, avez-vous des exemples d\'autres pays qui l\'ont fait ?',
                'created_at': (datetime.now(timezone.utc) - timedelta(hours=3)).isoformat(),
                'upvotes': 12,
                'downvotes': 1
            }
        ]
        
        return jsonify(proposal_with_comments)
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/proposals', methods=['POST'])
def create_proposal():
    """Crée une nouvelle proposition citoyenne"""
    try:
        data = request.get_json()
        
        # Validation des champs requis
        required_fields = ['title', 'description', 'full_text', 'category', 'author_id', 'author_name']
        if not all(field in data for field in required_fields):
            return jsonify({'error': 'Champs requis manquants'}), 400
        
        # Créer la nouvelle proposition
        new_proposal = {
            'id': f'prop-{str(uuid.uuid4())[:8]}',
            'title': data['title'],
            'description': data['description'],
            'full_text': data['full_text'],
            'author_id': data['author_id'],
            'author_name': data['author_name'],
            'category': data['category'],
            'status': 'Brouillon',
            'created_at': datetime.now(timezone.utc).isoformat(),
            'updated_at': datetime.now(timezone.utc).isoformat(),
            'supporters': 1,  # L'auteur est le premier supporter
            'signatures_required': data.get('signatures_required', 5000),
            'comments_count': 0,
            'upvotes': 1,
            'downvotes': 0,
            'tags': data.get('tags', []),
            'visibility': data.get('visibility', 'public'),
            'estimated_budget': data.get('estimated_budget', 0),
            'implementation_timeline': data.get('implementation_timeline', 'À déterminer')
        }
        
        citizen_proposals.append(new_proposal)
        
        # Enregistrer dans l'audit
        log_audit_event(
            user_id=data['author_id'],
            user_name=data['author_name'],
            action_type='PROPOSAL_CREATED',
            action_description=f'Nouvelle proposition créée : {data["title"]}',
            details={
                'proposal_id': new_proposal['id'],
                'category': data['category'],
                'title': data['title'][:100]
            }
        )
        
        # Notification aux administrateurs
        broadcast_notification(
            'new_proposal',
            '📝 Nouvelle Proposition',
            f'{data["author_name"]} a créé une nouvelle proposition : "{data["title"]}"',
            data={'proposal_id': new_proposal['id'], 'author': data['author_name']},
            target_role='administrator'
        )
        
        return jsonify({
            'message': 'Proposition créée avec succès',
            'proposal': new_proposal
        }), 201
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/proposals/<proposal_id>/support', methods=['POST'])
def support_proposal(proposal_id):
    """Soutenir une proposition"""
    try:
        data = request.get_json()
        user_id = data.get('user_id')
        user_name = data.get('user_name')
        
        if not user_id or not user_name:
            return jsonify({'error': 'user_id et user_name requis'}), 400
        
        proposal = next((p for p in citizen_proposals if p['id'] == proposal_id), None)
        if not proposal:
            return jsonify({'error': 'Proposition non trouvée'}), 404
        
        # Simuler l'ajout du support (dans une vraie app, vérifier les doublons)
        proposal['supporters'] += 1
        proposal['upvotes'] += 1
        proposal['updated_at'] = datetime.now(timezone.utc).isoformat()
        
        # Vérifier si le seuil de signatures est atteint
        if proposal['supporters'] >= proposal['signatures_required'] and proposal['status'] == 'Collecte signatures':
            proposal['status'] = 'En révision'
            
            # Notification de changement de statut
            broadcast_notification(
                'proposal_milestone',
                '🎉 Seuil Atteint !',
                f'La proposition "{proposal["title"]}" a atteint {proposal["signatures_required"]} signatures !',
                data={'proposal_id': proposal_id, 'supporters': proposal['supporters']},
                target_role='administrator'
            )
        
        # Enregistrer dans l'audit
        log_audit_event(
            user_id=user_id,
            user_name=user_name,
            action_type='PROPOSAL_SUPPORTED',
            action_description=f'Soutien à la proposition : {proposal["title"]}',
            details={
                'proposal_id': proposal_id,
                'new_supporters_count': proposal['supporters']
            }
        )
        
        return jsonify({
            'message': 'Support ajouté avec succès',
            'supporters': proposal['supporters'],
            'status': proposal['status']
        })
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/proposals/<proposal_id>/comments', methods=['POST'])
def add_proposal_comment(proposal_id):
    """Ajouter un commentaire à une proposition"""
    try:
        data = request.get_json()
        
        required_fields = ['author_name', 'content']
        if not all(field in data for field in required_fields):
            return jsonify({'error': 'Champs requis manquants'}), 400
        
        proposal = next((p for p in citizen_proposals if p['id'] == proposal_id), None)
        if not proposal:
            return jsonify({'error': 'Proposition non trouvée'}), 404
        
        # Créer le commentaire
        comment = {
            'id': f'comment-{str(uuid.uuid4())[:8]}',
            'author': data['author_name'],
            'content': data['content'],
            'created_at': datetime.now(timezone.utc).isoformat(),
            'upvotes': 0,
            'downvotes': 0
        }
        
        # Mettre à jour le compteur de commentaires
        proposal['comments_count'] += 1
        proposal['updated_at'] = datetime.now(timezone.utc).isoformat()
        
        # Enregistrer dans l'audit
        log_audit_event(
            user_id=data.get('user_id', 'anonymous'),
            user_name=data['author_name'],
            action_type='PROPOSAL_COMMENT',
            action_description=f'Commentaire ajouté sur : {proposal["title"]}',
            details={
                'proposal_id': proposal_id,
                'comment_preview': data['content'][:100]
            }
        )
        
        return jsonify({
            'message': 'Commentaire ajouté avec succès',
            'comment': comment,
            'comments_count': proposal['comments_count']
        }), 201
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/proposals/categories', methods=['GET'])
def get_proposal_categories():
    """Récupère les catégories de propositions disponibles"""
    return jsonify({
        'categories': [
            'Transport',
            'Environnement', 
            'Éducation',
            'Santé',
            'Économie',
            'Justice',
            'Travail',
            'Logement',
            'Culture',
            'Numérique',
            'Sécurité',
            'Autre'
        ]
    })

if __name__ == '__main__':
    print("🚀 Démarrage du Mock Blockchain Server avec Notifications...")
    print("📡 API disponible sur: http://0.0.0.0:5000")
    print("🔍 Health check: http://0.0.0.0:5000/health")
    print("📖 Documentation: http://0.0.0.0:5000/")
    print("🔔 WebSocket notifications: ws://0.0.0.0:5000/socket.io/")
    
    # Démarrer avec SocketIO au lieu de Flask direct
    socketio.run(app, host='0.0.0.0', port=5000, debug=True, allow_unsafe_werkzeug=True)