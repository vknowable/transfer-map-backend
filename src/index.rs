use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

// use sqlx::Error;
use sqlx::{Pool, Postgres};

use tokio::time::{sleep, Duration};
use tendermint::block::Height;
use tendermint_rpc::{
  endpoint::block,
  Client, HttpClient
};
use namada_sdk::{
  proto::generated::types::Tx,
  core::types::transaction::TxType,
  borsh::BorshDeserialize,
};
use prost::Message;
use tokio::sync::mpsc;
use futures::future::try_join_all;

use crate::model::TxdataModel;


pub async fn get_block(block_height: u32, http_client: &HttpClient) -> Result<block::Response, tendermint_rpc::Error> {
  let result = http_client.block(Height::from(block_height)).await;
  match result {
    Ok(block) => {
      // println!("Got block: {:?}", block_height);
      Ok(block)
    }
    Err(e) => {
      // eprintln!("Block http fetch error: {:?}", e);
      Err(e)
    }
  }
}

//TODO: don't forget we need to get block_results also to see if tx was successful

pub async fn fetch_blocks(
  shutdown_fetching: Arc<AtomicBool>,
  fast_sync: Arc<AtomicBool>,
  start_height: i64,
  tx: mpsc::Sender<block::Response>,
  http_client: HttpClient) {

  const BATCH_SIZE: u32 = 5;
  let mut start_height: u32 = start_height.try_into().unwrap_or(1); //set to highest_indexed + 1
  println!("Starting indexing from height {}", start_height);
  let mut retry_attempts: u32 = 0;

  while !shutdown_fetching.load(Ordering::Relaxed) {
    // if fast_sync = false fetch_blocks_batch with length 1
    let num_to_fetch: u32;
    let base_delay: u64;
    match fast_sync.load(Ordering::Relaxed) {
      true => {
        num_to_fetch = BATCH_SIZE;
        base_delay = 10;
      }
      false => {
        num_to_fetch = 1;
        base_delay = 5000;
      }
    }
    match fetch_blocks_batch(start_height, num_to_fetch, &http_client).await {
      Ok(blocks) => {
        for (_, block) in blocks.into_iter().enumerate() {
          if let Err(err) = tx.send(block).await {
            eprintln!("Error sending block to channel: {}", err);
            retry_attempts += 1;
            break;
          }
          else {
            retry_attempts = 0;
          }
        }
      }
      Err(err) => {
        if err.to_string().contains("must be less than or equal to the current blockchain height") {
          println!("Reached chain tip, waiting for next block...");
          fast_sync.store(false, Ordering::Relaxed);
        }
        else {
          eprintln!("Http client encountered error fetching blocks: {}", err);
        }
        retry_attempts += 1;
      }
    };

    if retry_attempts == 0 {
      match fast_sync.load(Ordering::Relaxed) {
        true => start_height += BATCH_SIZE,
        false => start_height += 1
      }
    }

    // cap delay at 64 sec
    retry_attempts = retry_attempts.min(12);
    let delay = Duration::from_millis(base_delay) * 2u32.pow(retry_attempts);
    println!("Waiting to retry... attempt: {}; delay: {:?}", retry_attempts, delay);
    sleep(delay).await;
  }
}

async fn fetch_blocks_batch(start_height: u32, batch_size: u32, http_client: &HttpClient) -> Result<Vec<block::Response>, tendermint_rpc::Error> {
  let batch_queries = (start_height..start_height + batch_size)
    .map(|height| get_block(height, &http_client));

  try_join_all(batch_queries).await

}

pub async fn handle_stream(mut rx: mpsc::Receiver<block::Response>, db: Pool<Postgres>) {
  while let Some(block) = rx.recv().await {
    let height: i64 = block.block.header().height.into();
    println!("Processing block {}", height);
    if height % 100 == 0 {
      // update highest_indexed in db every so often
      match sqlx::query!(
        "UPDATE configuration SET value = $1 WHERE key = 'highest_indexed'",
        height
        ).execute(&db).await {
          Ok(_) => println!("Updated highest_indexed: {}", height),
          Err(e) => println!("Could not update highest_indexed: {}", e)
        }
    }

    let tx_array: &Vec<Vec<u8>> = block.block.data();
    // println!("Received block from stream: {:?}", tx_array);
    println!("Found {} transactions", tx_array.len());
    for tx in tx_array {
      let tx_data = Tx::decode(&tx[..]).unwrap();
      let decoded_tx = namada_sdk::core::proto::Tx::try_from_slice(&tx_data.data).expect("Failed to get tx data from bytes");
      match decoded_tx.header().tx_type {
        TxType::Protocol(payload) => println!("TxType: Protocol, {:?}", payload.tx),
        TxType::Raw => println!("TxType: Raw"),
        TxType::Wrapper(_) => println!("TxType: Wrapper"),
        TxType::Decrypted(_) => {
          println!("TxType: Decrypted");
          let tag: String = decoded_tx.sections.get(0)
            .and_then(|section| section.code_sec())
            .map(|code_sec| code_sec.tag)
            .flatten()
            .unwrap_or_else(|| "unknown".to_string());
          if tag == "tx_transfer.wasm" {
            let transfer_data = decoded_tx.sections[1].data().unwrap().data;
            let transfer = namada_sdk::core::types::token::Transfer::try_from_slice(&transfer_data).unwrap();
            let tx_hash = decoded_tx.raw_header_hash().to_string();
            let source = transfer.source.to_string();
            let destination = transfer.target.to_string();
            let token = transfer.token.to_string();
            let amount = transfer.amount.to_string();
            let timestamp = decoded_tx.header().timestamp.to_rfc3339();

            //write to db
            let query_result = sqlx::query_as!(
                TxdataModel,
                "INSERT INTO txdata (txhash,height,source,destination,token,amount,timestamp) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
                tx_hash,
                height,
                source,
                destination,
                token,
                amount,
                timestamp
              )
              .fetch_one(&db)
              .await;

            match query_result {
              Ok(_) => {
                println!("Added tx {:?} to db", tx_hash);
              }
              Err(e) => {
                if e.to_string().contains("duplicate key value violates unique constraint") {
                  println!("Tx {:?} already exists in db", tx_hash);
                }
              }
            }
          }
        }
      }
    }
  }
}