use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    let conn = manager.get_connection();
    let stmt = "ALTER TABLE subscriptions ADD COLUMN credits INT NOT NULL DEFAULT 0";
    if let Err(err) = conn.execute_unprepared(stmt).await {
      if is_duplicate_column(&err) {
        return Ok(());
      }
      return Err(DbErr::Custom(format!(
        "Migration failed for statement:\n{stmt}\nError: {err}"
      )));
    }
    Ok(())
  }

  async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
    Ok(())
  }
}

fn is_duplicate_column(err: &DbErr) -> bool {
  err.to_string().contains("1060")
}
