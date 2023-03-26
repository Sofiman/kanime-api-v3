use std::future::{Future, Ready, ready};
use std::pin::Pin;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::{
    HttpMessage, HttpResponse, Error, web,
    body::EitherBody,
    guard::{Guard, GuardContext},
    http::{header::HeaderValue, StatusCode},
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
};

use redis::AsyncCommands;
use anyhow::{anyhow, Result};
use log::warn;
use serde::{Deserialize, Serialize};
use crate::types::AppState;

const TOKEN_REDIS_KEY_PREFIX: &str = "tk";
const AUTHORIZATION_HEADER: &str = "Authorization";
const TOKEN_BASE_TYPE: &str = "Bearer";
const TOKEN_LENGTH: u8 = 42;

const NANOID_ALPHABET: [char; 64] = [
    '_', '-', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g',
    'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

pub fn pick_user_id(req: &ServiceRequest) -> String {
    if let Some(ses) = req.extensions().get::<Session>() {
        format!("<{:?}@{}>", ses.role, ses.user_id)
    } else {
        "<A>".to_string()
    }
}

fn validate_nanoid(str: &str, expected_len: u8) -> bool {
    str.len() == expected_len as usize && str.chars().all(|c| NANOID_ALPHABET.contains(&c))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Mod,
    Admin
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub token: String,
    pub expires_on: u64,
    pub user_id: String,
    pub role: Role
}

pub struct KanimeAuth;

// Middleware factory is `Transform` trait
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S, ServiceRequest> for KanimeAuth
    where
        S: Service<ServiceRequest, Response=ServiceResponse<B>, Error=Error> + 'static,
        S::Future: 'static,
        B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = KanimeAuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(KanimeAuthMiddleware { service: Rc::new(service) }))
    }
}

enum SessionResult {
    Valid(Session),
    Invalid(&'static str, StatusCode),
    Anonymous
}

pub struct KanimeAuthMiddleware<S> {
    service: Rc<S>,
}

impl<S> KanimeAuthMiddleware<S> {
    async fn get_session(app: web::Data<AppState>, req: &ServiceRequest) -> Result<SessionResult> {
        use SessionResult::*;
        if let Some(Ok(val)) = req.headers().get(AUTHORIZATION_HEADER).map(HeaderValue::to_str) {
            if let Some((TOKEN_BASE_TYPE, right)) = val.split_once(' ') {
                if !validate_nanoid(right, TOKEN_LENGTH) {
                    return Ok(Invalid("Bad token formatting", StatusCode::BAD_REQUEST));
                }

                let raw: Option<String> = app.redis.get_async_connection().await?
                    .get(format!("{TOKEN_REDIS_KEY_PREFIX}:{right}")).await
                    .map_err(|e| anyhow!("Get token from redis: {e}"))?;
                let Some(raw) = raw else {
                    return Ok(Invalid("Token is invalid or has expired", StatusCode::FORBIDDEN));
                };

                let session: Session = serde_json::from_str(&raw)?;
                let now = SystemTime::now().duration_since(UNIX_EPOCH)?
                    .as_millis() as u64;
                if session.expires_on <= now {
                    return Ok(Invalid("Token is invalid or has expired", StatusCode::FORBIDDEN));
                }
                return Ok(Valid(session));
            }
        }
        Ok(Anonymous)
    }
}

impl<S, B> Service<ServiceRequest> for KanimeAuthMiddleware<S>
    where
        S: Service<ServiceRequest, Response=ServiceResponse<B>, Error=Error> + 'static,
        S::Future: 'static,
        B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future =  Pin<Box<dyn Future<Output=Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        use SessionResult::*;
        let svc = self.service.clone();
        let app = req.app_data::<web::Data<AppState>>().unwrap().clone();
        Box::pin(async move {
            match Self::get_session(app, &req).await {
                Ok(Anonymous) => svc.call(req).await.map(ServiceResponse::map_into_left_body),
                Ok(Valid(session)) => {
                    req.extensions_mut().insert(session);
                    svc.call(req).await.map(ServiceResponse::map_into_left_body)
                },
                Ok(Invalid(msg, code)) => {
                    let res = HttpResponse::build(code).body(msg);
                    Ok(req.into_response(res.map_into_right_body()))
                },
                Err(e) => {
                    warn!("Could not authenticate request: {e}");
                    let res = HttpResponse::InternalServerError().body("Could not authenticate request");
                    Ok(req.into_response(res.map_into_right_body()))
                }
            }
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RequireRoleGuard(pub Role);

impl Guard for RequireRoleGuard {
    fn check(&self, req: &GuardContext) -> bool {
        let exts = req.req_data();
        let session: Option<&Session> = exts.get();
        matches!(session, Some(session) if session.role == self.0)
    }
}
