use std::sync::Arc;
use crate::database::DatabaseService;
use crate::persistent_database;

impl From<Arc<DatabaseService>> for persistent_database::DatabaseService {
    fn from(db: Arc<DatabaseService>) -> Self {
        // Créer une nouvelle instance de la base de données persistante
        // Ceci est un hack temporaire pour permettre la conversion
        // dans une vraie application, on devrait avoir un seul type de DatabaseService
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:./data/e_government.db".to_string());
        
        // Notez que ceci est un bloc d'attente et n'est pas idéal,
        // mais c'est nécessaire pour l'implémentation From qui ne peut pas être async
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            persistent_database::DatabaseService::new(&db_url).await.unwrap_or_else(|_| {
                // Fallback au mode mémoire en cas d'échec
                panic!("Impossible de créer une nouvelle instance de DatabaseService")
            })
        })
    }
}