# Correction du problème SQLite en mémoire

## Problème

Lorsqu'on utilise une base de données SQLite en mémoire (:memory:), les tables créées ne sont 
disponibles que pour la connexion qui les a créées. Chaque nouvelle connexion obtient une base 
de données en mémoire complètement vide, même si des tables ont été créées auparavant.

Cela provoque l'erreur 
o such table: users même si les tables ont été créées avec succès dans 
la méthode DatabaseService::new().

## Solution

La solution consiste à utiliser une URL SQLite spéciale pour partager la base de données en mémoire 
entre toutes les connexions:

`
sqlite:file:memdb1?mode=memory&cache=shared
`

Au lieu de:

`
sqlite::memory:
`

## Comment appliquer la correction

Modifier la méthode DatabaseService::new() dans persistent_database.rs comme suit:

1. Remplacer la création du pool en mode mémoire par:

`ust
let pool = if is_memory_db {
    println!("Utilisation d'une base de données SQLite en mémoire PARTAGÉE");
    
    // URL pour une base de données en mémoire partagée entre les connexions
    let shared_memory_url = "file:memdb1?mode=memory&cache=shared";
    
    // Options spécifiques pour SQLite en mémoire partagée
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(shared_memory_url)  // URL partagée au lieu de :memory:
        .create_if_missing(true)
        .foreign_keys(true);
        
    SqlitePool::connect_with(options).await?
} else {
    // Options pour les bases de données sur fichier...
}
`

## Explication technique

SQLite stocke les bases de données en mémoire dans un espace global, mais par défaut, chaque connexion 
a son propre espace isolé. En utilisant ile:memdb1?mode=memory&cache=shared, on indique à SQLite de:

1. Créer une base de données en mémoire (mode=memory)
2. L'associer à un nom spécifique (memdb1)
3. La partager entre toutes les connexions (cache=shared)

Cette approche garantit que toutes les connexions accèdent à la même base de données en mémoire, et 
que les tables créées par une connexion sont accessibles par toutes les autres.

## Validation

Cette solution a été testée et validée avec un test minimal qui démontre que:

1. Une table créée avec une connexion directe est visible via un pool SQLx séparé
2. Les données insérées avec une connexion sont accessibles avec une autre connexion

## Références

- [Documentation SQLite URI](https://www.sqlite.org/uri.html)
- [SQLite In-Memory Databases](https://www.sqlite.org/inmemorydb.html)
