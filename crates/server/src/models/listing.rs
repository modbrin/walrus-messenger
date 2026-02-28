use serde::Deserialize;

use crate::error::{RequestError, ValidationError};
use crate::models::message::MessageId;
use crate::server::constants::MAX_LISTING_ELEMENTS;
pub const DEFAULT_LIMIT: i32 = 100;
pub const DEFAULT_PAGE: i32 = 1;

#[derive(Debug, Deserialize)]
pub struct ListingQuery {
    pub limit: Option<i32>,
    pub page: Option<i32>,
    pub offset: Option<MessageId>,
}

#[derive(Debug)]
pub enum ListingMode {
    Page { limit: i32, page: i32 },
    Offset { offset: MessageId, limit: i32 },
}

pub fn validate_limit(limit: i32) -> Result<(), RequestError> {
    if limit < 1 {
        return Err(ValidationError::InvalidInput {
            value: limit.to_string(),
            reason: "limit should be >= 1".to_string(),
        }
        .into());
    }
    if limit > MAX_LISTING_ELEMENTS {
        return Err(ValidationError::LimitExceeded {
            subject: "listing limit".to_string(),
            unit: "element".to_string(),
            attempted: limit as usize,
            limit: MAX_LISTING_ELEMENTS as usize,
        }
        .into());
    }
    Ok(())
}

pub fn validate_page(page: i32) -> Result<(), RequestError> {
    if page < 1 {
        return Err(ValidationError::InvalidInput {
            value: page.to_string(),
            reason: "page should be >= 1".to_string(),
        }
        .into());
    }
    Ok(())
}

pub fn validate_message_offset(offset: MessageId) -> Result<(), RequestError> {
    if offset < 0 {
        return Err(ValidationError::InvalidInput {
            value: offset.to_string(),
            reason: "offset should be >= 0".to_string(),
        }
        .into());
    }
    Ok(())
}

impl ListingMode {
    pub fn from_query(query: ListingQuery) -> Result<Self, RequestError> {
        let limit = query.limit.unwrap_or(DEFAULT_LIMIT);
        validate_limit(limit)?;
        if let Some(offset) = query.offset {
            if query.page.is_some() {
                return Err(ValidationError::InvalidInput {
                    value: "page".to_string(),
                    reason: "page cannot be used with offset mode".to_string(),
                }
                .into());
            }
            validate_message_offset(offset)?;
            Ok(Self::Offset { offset, limit })
        } else {
            let page = query.page.unwrap_or(DEFAULT_PAGE);
            validate_page(page)?;
            Ok(Self::Page { limit, page })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_query_uses_defaults_for_page_mode() {
        let mode = ListingMode::from_query(ListingQuery {
            limit: None,
            page: None,
            offset: None,
        })
        .unwrap();

        match mode {
            ListingMode::Page { limit, page } => {
                assert_eq!(limit, DEFAULT_LIMIT);
                assert_eq!(page, DEFAULT_PAGE);
            }
            ListingMode::Offset { .. } => panic!("expected page mode"),
        }
    }

    #[test]
    fn from_query_parses_offset_mode() {
        let mode = ListingMode::from_query(ListingQuery {
            limit: Some(25),
            page: None,
            offset: Some(42),
        })
        .unwrap();

        match mode {
            ListingMode::Offset { offset, limit } => {
                assert_eq!(offset, 42);
                assert_eq!(limit, 25);
            }
            ListingMode::Page { .. } => panic!("expected offset mode"),
        }
    }

    #[test]
    fn from_query_rejects_offset_with_page() {
        let err = ListingMode::from_query(ListingQuery {
            limit: Some(25),
            page: Some(2),
            offset: Some(42),
        })
        .expect_err("expected invalid input error");

        assert!(matches!(
            err,
            RequestError::Validation(ValidationError::InvalidInput { value, .. }) if value == "page"
        ));
    }

    #[test]
    fn from_query_rejects_invalid_limit() {
        let err = ListingMode::from_query(ListingQuery {
            limit: Some(0),
            page: Some(1),
            offset: None,
        })
        .expect_err("expected invalid input error");

        assert!(matches!(
            err,
            RequestError::Validation(ValidationError::InvalidInput { value, .. }) if value == "0"
        ));
    }

    #[test]
    fn from_query_rejects_page_below_one() {
        let err = ListingMode::from_query(ListingQuery {
            limit: Some(5),
            page: Some(0),
            offset: None,
        })
        .expect_err("expected invalid input error");

        assert!(matches!(
            err,
            RequestError::Validation(ValidationError::InvalidInput { value, .. }) if value == "0"
        ));
    }

    #[test]
    fn from_query_rejects_negative_offset() {
        let err = ListingMode::from_query(ListingQuery {
            limit: Some(10),
            page: None,
            offset: Some(-1),
        })
        .expect_err("expected invalid input error");

        assert!(matches!(
            err,
            RequestError::Validation(ValidationError::InvalidInput { value, .. }) if value == "-1"
        ));
    }
}
