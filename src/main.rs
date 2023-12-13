mod handler;
mod model;
mod route;
mod schema;
mod index;

use index::fetch_blocks;
use model::ConfigurationModel;
use tendermint_rpc::{HttpClient, Url, endpoint::block};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::str::FromStr;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use axum::http::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    HeaderValue, Method,
};
use dotenv::dotenv;
use route::create_router;
use tower_http::cors::CorsLayer;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

pub struct AppState {
    db: Pool<Postgres>,
    http_client: HttpClient,
    shutdown_fetching: Arc<AtomicBool>,
    fast_sync: Arc<AtomicBool>,
    fetching_task: Option<tokio::task::JoinHandle<()>>,
    highest_indexed: i64,
    // tx: mpsc::Sender<(u32, String)>,
}

impl Drop for AppState {
    fn drop(&mut self) {
        // stop the block fetching task
        self.shutdown_fetching.store(true, Ordering::Relaxed);

        if let Some(fetching_task) = self.fetching_task.take() {
            let _ = tokio::runtime::Handle::current().block_on(fetching_task);
        }
    }
}

impl AppState {
    pub async fn new(db: Pool<Postgres>, http_client: HttpClient, tx: mpsc::Sender<block::Response>) -> Self {
        let shutdown_fetching = Arc::new(AtomicBool::new(false));
        let fast_sync = Arc::new(AtomicBool::new(true));
        //get highest indexed
        let query_result = sqlx::query_as!(
            ConfigurationModel,
            "SELECT * FROM configuration WHERE key = 'highest_indexed'"
        )
        .fetch_one(&db)
        .await;
        
        let highest_indexed: i64;
        match query_result {
            Ok(height) => highest_indexed = height.value.unwrap(),
            Err(_) => highest_indexed = 0,
        }

        // handle to block fetching task
        let fetching_task = tokio::spawn(fetch_blocks(
            shutdown_fetching.clone(),
            fast_sync.clone(),
            highest_indexed + 1,
            tx.clone(),
            http_client.clone()
        ));

        AppState { 
            db,
            http_client,
            shutdown_fetching,
            fast_sync,
            fetching_task: Some(fetching_task),
            highest_indexed,
            // tx,
        }
    }

    // TODO: methods to start/stop indexing
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = match PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            println!("Connected to database...");
            pool
        }
        Err(err) => {
            println!("Failed to connect to database: {:?}", err);
            std::process::exit(1);
        }
    };

    let rpc: &str = &std::env::var("RPC").expect("RPC must be set");
    let url = Url::from_str(rpc).expect("invalid RPC address");
    let http_client = HttpClient::new(url).unwrap();
    let listen_port = std::env::var("LISTEN_PORT").unwrap_or_else(|_| String::from("8000"));
    let listen_addr = format!("0.0.0.0:{}", listen_port);
    let allow_from = std::env::var("ALLOW_FROM").unwrap_or_else(|_| String::from("http://localhost:3001"));

    let listener = TcpListener::bind(&listen_addr).await.unwrap();
    let cors = CorsLayer::new()
        .allow_origin([
            format!("http://localhost:{}", listen_port).parse::<HeaderValue>().unwrap(),
            allow_from.parse::<HeaderValue>().unwrap(),
        ])
        .allow_methods([Method::GET])
        .allow_credentials(true)
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);
    
    let (tx, rx) = mpsc::channel::<block::Response>(100);
    let app_state = Arc::new(AppState::new(pool.clone(), http_client.clone(), tx).await);
    tokio::spawn(index::handle_stream(rx, app_state.db.clone()));

    let app = create_router(app_state).layer(cors);

    println!("🚀 Server started successfully");
    axum::serve(listener, app).await.unwrap();

}
