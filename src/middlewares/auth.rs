use std::future::{Future, Ready, ready};
use std::pin::Pin;
use std::rc::Rc;
use actix_web::{Error, error::ErrorForbidden};
use actix_web::{dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform}, HttpMessage, web};
use actix_web::guard::{Guard, GuardContext};
use actix_web::http::header::HeaderValue;
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = KanimeAuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(KanimeAuthMiddleware { service: Rc::new(service) }))
    }
}

pub struct KanimeAuthMiddleware<S> {
    service: Rc<S>,
}

impl<S> KanimeAuthMiddleware<S> {
    async fn get_session(app: web::Data<AppState>, req: &ServiceRequest) -> Result<Option<Session>> {
        if let Some(Ok(val)) = req.headers().get(AUTHORIZATION_HEADER).map(HeaderValue::to_str) {
            if let Some((TOKEN_BASE_TYPE, right)) = val.split_once(" ") {
                if !validate_nanoid(right, TOKEN_LENGTH) {
                    return Ok(None); // Bad token formatting
                }
                let raw: String = app.redis.get_async_connection()
                    .await?
                    .get(format!("{TOKEN_REDIS_KEY_PREFIX}:{right}"))
                    .await
                    .map_err(|e| anyhow!("Get token from redis: {e}"))?;
                let session: Session = serde_json::from_str(&raw)
                    .map_err(|e| anyhow!("Deserialize redis result: {e}"))?;
                // TODO: Check expires_on on session
                return Ok(Some(session));
            }
        }
        Ok(None)
    }
}

impl<S, B> Service<ServiceRequest> for KanimeAuthMiddleware<S>
    where
        S: Service<ServiceRequest, Response=ServiceResponse<B>, Error=Error> + 'static,
        S::Future: 'static,
        B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future =  Pin<Box<dyn Future<Output=Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        Box::pin(async move {
            let app = req.app_data::<web::Data<AppState>>().unwrap().clone();
            match Self::get_session(app, &req).await {
                Ok(Some(session)) => {
                    req.extensions_mut().insert(session);
                    svc.call(req).await
                },
                Ok(None) => svc.call(req).await,
                Err(e) => {
                    warn!("Could not authenticate request: {e}");
                    Err(ErrorForbidden("Could not authenticate request"))?
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
        match session {
            Some(session) if session.role == self.0 => true,
            _ => false
        }
    }
}
