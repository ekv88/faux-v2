use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    let conn = manager.get_connection();
    let statements = [
      // Indexes for faster lookups / phpMyAdmin relations view
      "ALTER TABLE links ADD INDEX idx_links_user_id (user_id)",
      "ALTER TABLE `keys` ADD INDEX idx_keys_user_id (user_id)",
      "ALTER TABLE roles ADD INDEX idx_roles_user_id (user_id)",
      "ALTER TABLE screen_results ADD INDEX idx_screen_results_user_id (user_id)",
      "ALTER TABLE settings ADD INDEX idx_settings_user_id (user_id)",
      "ALTER TABLE subscriptions ADD INDEX idx_subscriptions_user_id (user_id)",
      "ALTER TABLE subscriptions ADD INDEX idx_subscriptions_package_id (package_id)",
      "ALTER TABLE subscriptions ADD INDEX idx_subscriptions_payment_id (payment_id)",
      // Foreign keys
      "ALTER TABLE links ADD CONSTRAINT fk_links_user_id FOREIGN KEY (user_id) REFERENCES users(id)",
      "ALTER TABLE `keys` ADD CONSTRAINT fk_keys_user_id FOREIGN KEY (user_id) REFERENCES users(id)",
      "ALTER TABLE roles ADD CONSTRAINT fk_roles_user_id FOREIGN KEY (user_id) REFERENCES users(id)",
      "ALTER TABLE screen_results ADD CONSTRAINT fk_screen_results_user_id FOREIGN KEY (user_id) REFERENCES users(id)",
      "ALTER TABLE settings ADD CONSTRAINT fk_settings_user_id FOREIGN KEY (user_id) REFERENCES users(id)",
      "ALTER TABLE subscriptions ADD CONSTRAINT fk_subscriptions_user_id FOREIGN KEY (user_id) REFERENCES users(id)",
      "ALTER TABLE subscriptions ADD CONSTRAINT fk_subscriptions_package_id FOREIGN KEY (package_id) REFERENCES packages(id)",
      "ALTER TABLE subscriptions ADD CONSTRAINT fk_subscriptions_payment_id FOREIGN KEY (payment_id) REFERENCES payments(id)",
    ];

    for stmt in statements {
      if let Err(err) = conn.execute_unprepared(stmt).await {
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
