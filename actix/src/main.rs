use actix_web::{web, App, Error, HttpResponse, HttpServer};
use log::{debug, error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
struct ProxyRequest {
    url: String,
    method: String,
    body: Option<String>,
    headers: Option<HashMap<String, String>>,
}

#[derive(Serialize, Debug)]
struct ProxyResponse {
    status: u16,
    body: String,
    headers: HashMap<String, String>,
}

async fn proxy_handler(req: web::Json<ProxyRequest>) -> Result<HttpResponse, Error> {
    info!("Received proxy request for URL: {}", req.url);
    debug!("Request details: {:?}", req);

    let client = Client::new();

    let mut request = match req.method.to_uppercase().as_str() {
        "GET" => client.get(&req.url),
        "POST" => client.post(&req.url),
        "PUT" => client.put(&req.url),
        "DELETE" => client.delete(&req.url),
        _ => {
            error!("Unsupported HTTP method: {}", req.method);
            return Ok(HttpResponse::BadRequest().body("Unsupported HTTP method"));
        }
    };

    // Add headers if provided
    if let Some(headers) = &req.headers {
        for (key, value) in headers {
            request = request.header(key, value);
        }
    }

    // Add body if provided
    if let Some(body) = &req.body {
        request = request.body(body.clone());
    }

    let response = match request.send().await {
        Ok(resp) => resp,
        Err(e) => {
            error!("Failed to send request: {}", e);
            return Ok(HttpResponse::InternalServerError().body("Failed to send request"));
        }
    };

    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let body = match response.text().await {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to read response body: {}", e);
            return Ok(HttpResponse::InternalServerError().body("Failed to read response body"));
        }
    };

    // Convert HeaderMap to HashMap<String, String>
    let headers_map: HashMap<String, String> = headers
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let proxy_response = ProxyResponse {
        status,
        body,
        headers: headers_map,
    };

    info!("Proxy request completed. Status: {}", status);
    debug!("Response details: {:?}", proxy_response);

    Ok(HttpResponse::Ok().json(proxy_response))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    info!("Starting proxy server on 127.0.0.1:8080");
    HttpServer::new(|| App::new().route("/proxy", web::post().to(proxy_handler)))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
