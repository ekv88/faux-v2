use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    let sql = include_str!("../sql/001_init.sql");
    let cleaned = strip_sql_comments(sql);
    for statement in cleaned.split(';') {
      let stmt = statement.trim();
      if stmt.is_empty() {
        continue;
      }
      eprintln!("Applying SQL:\n{stmt}\n");
      if let Err(err) = manager.get_connection().execute_unprepared(stmt).await {
        return Err(DbErr::Custom(format!(
          "Migration failed for statement:\n{stmt}\nError: {err}"
        )));
      }
    }
    Ok(())
  }

  async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
    Ok(())
  }
}

fn strip_sql_comments(sql: &str) -> String {
  let mut out = String::new();
  for line in sql.lines() {
    let trimmed = line.trim();
    if trimmed.starts_with("--") || trimmed.is_empty() {
      continue;
    }
    out.push_str(line);
    out.push('\n');
  }
  out
}
