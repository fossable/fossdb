use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};

pub async fn auth_middleware(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = crate::auth::verify_jwt(token).map_err(|_| StatusCode::UNAUTHORIZED)?;

    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}

/// Optional auth middleware - doesn't fail if no auth header is present
/// Use this for endpoints that should work for both authenticated and unauthenticated users
pub async fn optional_auth_middleware(mut req: Request, next: Next) -> Response {
    // Try to extract auth header
    if let Some(auth_header) = req.headers().get(header::AUTHORIZATION)
        && let Ok(auth_str) = auth_header.to_str()
        && let Some(token) = auth_str.strip_prefix("Bearer ")
        && let Ok(claims) = crate::auth::verify_jwt(token)
    {
        // Insert claims into request extensions
        req.extensions_mut().insert(claims);
    }

    // Always proceed, whether auth succeeded or not
    next.run(req).await
}
