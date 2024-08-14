use axum::{
    extract::{Extension, Json},
    http::{Method, Request, Uri},
    response::IntoResponse,
    routing::post,
    Router,
};
use hyper::{Body, Client, StatusCode};
use serde_json::{to_vec, Value};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // 프록시 서버 주소 설정
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // Hyper HTTP 클라이언트 생성
    let client = Client::new();

    // 라우터 설정
    let app = Router::new()
        .route("/proxy", post(proxy_request))
        .layer(Extension(client));

    // 서버 시작
    println!("프록시 서버 시작: http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn proxy_request(
    Extension(client): Extension<Client<hyper::client::connect::HttpConnector>>,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // 요청 데이터 추출
    let url = payload["url"].as_str().ok_or((
        StatusCode::BAD_REQUEST,
        "url 필드가 필요합니다.".to_string(),
    ))?;
    let method = payload["method"].as_str().ok_or((
        StatusCode::BAD_REQUEST,
        "method 필드가 필요합니다.".to_string(),
    ))?;
    let headers = payload["headers"].as_object();
    let body = payload["body"].clone();

    // URL과 Method 파싱
    let uri: Uri = url.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "잘못된 URL 형식입니다.".to_string(),
        )
    })?;
    let method: Method = method.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "잘못된 메서드 형식입니다.".to_string(),
        )
    })?;

    // 요청 생성
    let mut req = Request::new(Body::from(to_vec(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("body를 직렬화 할 수 없습니다: {}", e),
        )
    })?));
    *req.method_mut() = method;
    *req.uri_mut() = uri;

    // 헤더 추가
    if let Some(headers) = headers {
        for (k, v) in headers.iter() {
            if let Some(v) = v.as_str() {
                req.headers_mut().insert(
                    hyper::header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                    hyper::header::HeaderValue::from_str(v).unwrap(),
                );
            }
        }
    }

    // 요청 전송 및 응답 반환
    let response = client
        .request(req)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(response)
}
