// pagination.rs - Keyset pagination for the admin list endpoints
//
// Same contract as kleido: `limit` plus an opaque `after` cursor in, and
// `{items, next_cursor}` out. Lists are ordered by primary key, so a walk
// that follows `next_cursor` visits every item once even while rows are
// being added; there is deliberately no `total`, which would cost a
// COUNT(*) per page.
use crate::errors::ApiError;
use serde::Serialize;
use utoipa::ToSchema;

pub const DEFAULT_LIMIT: i64 = 100;
pub const MAX_LIMIT: i64 = 1000;

pub fn default_limit() -> i64 {
    DEFAULT_LIMIT
}

pub fn check_limit(limit: i64) -> Result<(), ApiError> {
    if (1..=MAX_LIMIT).contains(&limit) {
        Ok(())
    } else {
        Err(ApiError::InvalidQuery(format!(
            "limit must be between 1 and {MAX_LIMIT}"
        )))
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
    /// Pass as `after` to get the next page; absent on the last page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

impl<T> ListResponse<T> {
    /// A full page implies there may be more; the last item's key is the
    /// cursor. A list whose size is a multiple of `limit` therefore ends with
    /// one empty page, the price of not fetching `limit + 1` rows.
    pub fn new(items: Vec<T>, limit: i64, key: impl Fn(&T) -> String) -> Self {
        let next_cursor = if items.len() as i64 == limit {
            items.last().map(key)
        } else {
            None
        };
        Self { items, next_cursor }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_bounds() {
        assert!(check_limit(1).is_ok());
        assert!(check_limit(1000).is_ok());
        assert!(check_limit(0).is_err());
        assert!(check_limit(1001).is_err());
        assert!(check_limit(-5).is_err());
    }

    #[test]
    fn cursor_only_on_full_page() {
        let full = ListResponse::new(vec!["a", "b"], 2, |s| s.to_string());
        assert_eq!(full.next_cursor.as_deref(), Some("b"));
        let partial = ListResponse::new(vec!["a"], 2, |s| s.to_string());
        assert!(partial.next_cursor.is_none());
        let json = serde_json::to_value(&partial).unwrap();
        assert!(json.get("next_cursor").is_none());
    }
}
