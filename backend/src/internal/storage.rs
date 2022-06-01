use rocket_db_pools::mongodb::Client;
use rocket_db_pools::Database;

#[derive(Database)]
#[database("storage")]
pub struct Storage(Client);
