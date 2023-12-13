use serde::{Deserialize, Serialize};
use sqlx::FromRow;
// use uuid::Uuid;

#[derive(Debug, FromRow, Deserialize, Serialize)]
pub struct TxdataModel {
  pub txhash: String,
  pub height: i64,
  pub source: String,
  pub destination: String,
  pub token: String,
  pub amount: String,
  pub timestamp: String,
}

#[derive(Debug, FromRow, Deserialize, Serialize)]
pub struct ConfigurationModel {
  pub key: String,
  pub value: Option<i64>
}