use anyhow::Result;
use api_gateway::persistent_database::DatabaseService;
use sqlx::Row;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Testing DatabaseService with in-memory SQLite database");
    println!("DATABASE_URL: {}", std::env::var("DATABASE_URL").unwrap_or_default());
    
    // Create the database service
    println!("⚙️  Creating DatabaseService...");
    let db = DatabaseService::new("sqlite::memory:").await?;
    
    // Check if tables exist by querying sqlite_master
    println!("📋 Checking if tables were created:");
    let tables = sqlx::query("SELECT name FROM sqlite_master WHERE type='table'")
        .fetch_all(db.pool())
        .await?;
    
    println!("Found {} tables:", tables.len());
    for row in tables {
        let table_name: String = row.get("name");
        println!(" - {}", table_name);
    }
    
    // Try to insert a test user
    println!("👤 Testing user creation...");
    match db.create_user("test@example.com", "Test User", "password123").await {
        Ok(user) => println!("✅ User created successfully with ID: {}", user.id),
        Err(e) => println!("❌ Error creating user: {}", e),
    }
    
    // Try to fetch users
    println!("🔎 Querying users table...");
    match sqlx::query("SELECT * FROM users").fetch_all(db.pool()).await {
        Ok(rows) => println!("✅ Found {} users in the database", rows.len()),
        Err(e) => println!("❌ Error querying users: {}", e),
    }
    
    println!("✅ Test completed successfully!");
    Ok(())
}
