use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "screen_results")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub id: String,
  pub user_id: Option<String>,
  pub file_name: String,
  pub debug: Option<Json>,
  pub c_time: Option<DateTimeUtc>,
  pub e_time: Option<DateTimeUtc>,
  pub status: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
