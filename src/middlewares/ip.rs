use std::future::{Ready, ready};
use std::net::{IpAddr, SocketAddr};
use actix_web::{dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform}, Error};
use anyhow::{anyhow, Result};

const CLOUDFLARE_IP_HEADER: &str = "CF-Connecting-IP";

pub struct CloudflareClientIp;

// Middleware factory is `Transform` trait
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S, ServiceRequest> for CloudflareClientIp
    where
        S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
        S::Future: 'static,
        B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = CloudflareClientIpMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(CloudflareClientIpMiddleware { service }))
    }
}

pub struct CloudflareClientIpMiddleware<S> {
    service: S,
}

impl<S> CloudflareClientIpMiddleware<S> {
    fn header_value_to_ip(req: &ServiceRequest) -> Result<SocketAddr> {
        let ip = req.headers().get(CLOUDFLARE_IP_HEADER)
            .ok_or_else(|| anyhow!("No cloudflare IP header"))?;
        let peer_addr: IpAddr = ip.to_str()?.parse()?;
        let local = req.peer_addr().ok_or_else(|| anyhow!("No peer addr"))?.port();
        Ok(SocketAddr::new(peer_addr, local))
    }
}

impl<S, B> Service<ServiceRequest> for CloudflareClientIpMiddleware<S>
    where
        S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
        S::Future: 'static,
        B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = S::Future;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        if let Ok(ip) = Self::header_value_to_ip(&req) {
            req.head_mut().peer_addr = Some(ip);
        }

        self.service.call(req)
    }
}
