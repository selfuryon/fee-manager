// extract.rs - Request extractors whose rejections are `ApiError`s
//
// Axum's own Json/Query/Path reject with plain-text bodies; these wrappers
// route every rejection through ApiError so clients always get the JSON
// error body (see errors.rs for the status and code mapping).
use crate::errors::ApiError;
use axum::extract::{FromRequest, FromRequestParts};

#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(ApiError))]
pub struct AppJson<T>(pub T);

#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Query), rejection(ApiError))]
pub struct AppQuery<T>(pub T);

#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(ApiError))]
pub struct AppPath<T>(pub T);
