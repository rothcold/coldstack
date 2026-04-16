use actix_web::HttpResponse;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Db(rusqlite::Error),
    Pool(r2d2::Error),
    NotFound,
    BadRequest(String),
    Conflict(String),
    Forbidden(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Db(e) => write!(f, "Database error: {}", e),
            AppError::Pool(e) => write!(f, "Connection pool error: {}", e),
            AppError::NotFound => write!(f, "Not found"),
            AppError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            AppError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
        }
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Db(e)
    }
}

impl From<r2d2::Error> for AppError {
    fn from(e: r2d2::Error) -> Self {
        AppError::Pool(e)
    }
}

impl AppError {
    pub fn to_response(&self) -> HttpResponse {
        match self {
            AppError::Db(_) | AppError::Pool(_) => HttpResponse::InternalServerError().finish(),
            AppError::NotFound => HttpResponse::NotFound().finish(),
            AppError::BadRequest(msg) => HttpResponse::BadRequest().body(msg.clone()),
            AppError::Conflict(msg) => HttpResponse::Conflict().body(msg.clone()),
            AppError::Forbidden(msg) => HttpResponse::Forbidden().body(msg.clone()),
        }
    }
}
