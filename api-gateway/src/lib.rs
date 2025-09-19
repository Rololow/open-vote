// Bibliothèque API Gateway pour les tests et l'intégration externe
pub mod auth;
pub mod gateway;
pub mod middleware;
pub mod proxy;
pub mod routes;
pub mod models;
pub mod database;
pub mod identity_validation;
pub mod user_management;

// Nouveaux modules pour le système persistant
pub mod persistent_database;
pub mod crypto_service;
pub mod persistent_routes;
pub mod persistent_gateway;
pub mod database_utils;
pub mod handler_utils;