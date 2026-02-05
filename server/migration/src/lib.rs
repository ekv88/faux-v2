use sea_orm_migration::prelude::*;

mod m20260205_000001_init;
mod m20260205_000002_relations;
mod m20260205_000003_add_credits;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
  fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![
      Box::new(m20260205_000001_init::Migration),
      Box::new(m20260205_000002_relations::Migration),
      Box::new(m20260205_000003_add_credits::Migration),
    ]
  }
}
