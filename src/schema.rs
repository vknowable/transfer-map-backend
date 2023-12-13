use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Default)]
pub struct FilterOptions {
  pub page: Option<usize>,
  pub limit: Option<usize>,
}

#[derive(Deserialize, Debug)]
pub struct ParamOptions {
  pub source: Option<String>,
  pub destination: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTxdataSchema {
  pub txhash: String,
  pub height: i64,
  pub source: String,
  pub destination: String,
  pub token: String,
  pub amount: String,
  pub timestamp: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateTxdataSchema {
  pub txhash: Option<String>,
  pub height: Option<i64>,
  pub source: Option<String>,
  pub destination: Option<String>,
  pub token: Option<String>,
  pub amount: Option<String>,
  pub timemstamp: Option<String>
}