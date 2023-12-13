# Namada Transfer indexer/api server

App to index transfers to postgres and serve them to a json api endpoint.

## Usage
1. Start the postgres container:
```
cd docker-compose
docker compose -f postgres-docker-compose.yml up -d
```
2. Run db migrations
```
cargo install sqlx-cli
sqlx migrate run
```
3. In the .env file, set `RPC` to the url of your node/rpc, `LISTEN_PORT` to the port for the server to listen on, and `ALLOW_FROM` to the url of your front-end app. Then, start the indexer/server:
```
cargo run
```
4. Get recent tx's
```
curl localhost:8000/api/txdata
```

## ToDo/Known issues:
- doesn't check if tx was successful before indexing... need to check `block_results` endpoint at corresponding height and check by tx hash to see if successful
- api only serves 100 txs ordered by txhash; needs to be ordered by timestamp and support for pagination added
- support could be added for parameters (eg: all transfers where source='tnam...')
- only transfers (transparent) are indexed; all other transaction types are ignored