use std::net::SocketAddr;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};
use serde_json::json;
use tokio::fs;

async fn handle_http(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path().to_string();
    let method = req.method();

    match (method, path.as_str()) {
        // Health / OPTIONS / HEAD
        (&Method::OPTIONS, _) | (&Method::HEAD, _) => {
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::empty())
                .unwrap())
        }

        // Favicon
        (&Method::GET, "/favicon.ico") => {
            Ok(Response::new(Body::from("")))
        }

        // Example: Peer ANR by pk
        (Method::GET, path) if path.starts_with("/api/peer/anr/") => {
            let pk = path.trim_start_matches("/api/peer/anr/");
            // Call your API handler here:
            let anr = api_peer_anr_by_pk(pk).await;
            let body = json!({ "error": "ok", "anr": anr });
            Ok(json_response(body))
        }

        // Example: Chain tip
        (&Method::GET, "/api/chain/tip") => {
            let tip = api_chain_entry_tip().await;
            Ok(json_response(tip))
        }

        // Example: Wallet balance
        (Method::GET, path) if path.starts_with("/api/wallet/balance/") => {
            let pk = path.trim_start_matches("/api/wallet/balance/");
            let balance = api_wallet_balance(pk).await;
            let body = json!({ "error": "ok", "balance": balance });
            Ok(json_response(body))
        }

        // Example: Tx submit (POST)
        (Method::POST, "/api/tx/submit") => {
            let whole_body = hyper::body::to_bytes(req.into_body()).await.unwrap();
            let tx_packed = String::from_utf8_lossy(&whole_body).trim().to_string();
            let result = api_tx_submit(&tx_packed).await;
            Ok(json_response(result))
        }

        // Default 404
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(r#"{"error":"not_found"}"#))
            .unwrap()),
    }
}

fn json_response<T: serde::Serialize>(val: T) -> Response<Body> {
    Response::builder()
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .status(StatusCode::OK)
        .body(Body::from(serde_json::to_string(&val).unwrap()))
        .unwrap()
}

async fn api_peer_anr_by_pk(pk: &str) -> serde_json::Value {
    json!({ "pk": pk, "node": "example" })
}

async fn api_chain_entry_tip() -> serde_json::Value {
    json!({ "height": 123456, "hash": "abc123" })
}

async fn api_wallet_balance(pk: &str) -> serde_json::Value {
    json!({ "pk": pk, "symbol": "AMA", "balance": 1000 })
}

async fn api_tx_submit(tx: &str) -> serde_json::Value {
    json!({ "submitted": true, "tx": tx })
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 4000));

    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, hyper::Error>(service_fn(handle_http))
    });

    let server = Server::bind(&addr).serve(make_svc);

    println!("Ama.MultiServer listening on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
