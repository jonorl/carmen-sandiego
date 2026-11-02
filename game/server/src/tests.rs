//! Each test gets a fresh database with the migrations applied (sqlx::test),
//! which needs DATABASE_URL pointing at a Postgres it may create databases on.

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use carmen_shared::{Detective, MAX_ARRESTS};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

const GAME: &str = "https://game.example";

async fn send(pool: &PgPool, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, String) {
    let request = Request::builder().method(method).uri(uri).header(header::ORIGIN, GAME);
    let request = match body {
        Some(json) => request.header(header::CONTENT_TYPE, "application/json").body(Body::from(json.to_string())),
        None => request.body(Body::empty()),
    }
    .unwrap();
    let response = super::app(pool.clone(), vec![GAME.parse().unwrap()]).oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

async fn detective(pool: &PgPool, method: &str, name: &str, arrests: Option<u32>) -> Detective {
    let (status, body) = send(pool, method, &format!("/detectives/{name}"), arrests.map(|n| serde_json::json!({ "arrests": n }))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    serde_json::from_str(&body).unwrap()
}

#[sqlx::test]
async fn health_checks_the_database(pool: PgPool) {
    assert_eq!(send(&pool, "GET", "/health", None).await, (StatusCode::OK, "ok".to_string()));
}

#[sqlx::test]
async fn a_new_detective_has_no_arrests(pool: PgPool) {
    assert_eq!(detective(&pool, "GET", "%20Jon%20", None).await, Detective { name: "Jon".into(), arrests: 0 });
}

#[sqlx::test]
async fn a_tally_never_goes_down(pool: PgPool) {
    assert_eq!(detective(&pool, "PUT", "Jon", Some(5)).await.arrests, 5);
    assert_eq!(detective(&pool, "PUT", "Jon", Some(3)).await.arrests, 5);
    assert_eq!(detective(&pool, "PUT", "Jon", Some(7)).await.arrests, 7);
    assert_eq!(detective(&pool, "GET", "Jon", None).await.arrests, 7);
}

#[sqlx::test]
async fn names_match_whatever_the_case(pool: PgPool) {
    detective(&pool, "PUT", "jon", Some(2)).await;
    // The name is kept as last sent, so a detective can fix their capitals.
    assert_eq!(detective(&pool, "PUT", "JON", Some(1)).await, Detective { name: "JON".into(), arrests: 2 });
    assert_eq!(detective(&pool, "GET", "Jon", None).await.arrests, 2);
}

#[sqlx::test]
async fn nonsense_is_turned_away(pool: PgPool) {
    assert_eq!(send(&pool, "GET", "/detectives/123456789", None).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(send(&pool, "GET", "/detectives/%20", None).await.0, StatusCode::BAD_REQUEST);
    let too_many = serde_json::json!({ "arrests": MAX_ARRESTS + 1 });
    assert_eq!(send(&pool, "PUT", "/detectives/Jon", Some(too_many)).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(detective(&pool, "GET", "Jon", None).await.arrests, 0);
}

#[sqlx::test]
async fn the_roster_lists_most_arrests_first(pool: PgPool) {
    detective(&pool, "PUT", "Ana", Some(1)).await;
    detective(&pool, "PUT", "Jon", Some(9)).await;
    detective(&pool, "PUT", "Bea", Some(1)).await;
    let (status, body) = send(&pool, "GET", "/detectives", None).await;
    assert_eq!(status, StatusCode::OK);
    let roster: Vec<Detective> = serde_json::from_str(&body).unwrap();
    let names: Vec<&str> = roster.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, ["Jon", "Ana", "Bea"]);
}

#[sqlx::test]
async fn only_the_game_may_call_from_a_browser(pool: PgPool) {
    let preflight = |origin: &'static str| {
        Request::builder()
            .method("OPTIONS")
            .uri("/detectives/Jon")
            .header(header::ORIGIN, origin)
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "PUT")
            .body(Body::empty())
            .unwrap()
    };
    let app = super::app(pool, vec![GAME.parse().unwrap()]);
    let allowed = app.clone().oneshot(preflight(GAME)).await.unwrap();
    assert_eq!(allowed.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], GAME);
    let other = app.oneshot(preflight("https://elsewhere.example")).await.unwrap();
    assert!(other.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
}
