//! src/routes/home/mod.rs

use actix_web::{HttpResponse, http::header};

pub async fn home() -> HttpResponse {
    HttpResponse::Ok()
        .insert_header(header::ContentType::html())
        .body(include_str!("home.html"))
}
