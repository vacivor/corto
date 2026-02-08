use sea_orm::{Database, DatabaseConnection};

use crate::config::config::DatasourceConfig;

pub async fn init_db(datasource: &DatasourceConfig) -> DatabaseConnection {
    let db = Database::connect(&datasource.url)
        .await
        .expect("failed to connect database");
    db
}
