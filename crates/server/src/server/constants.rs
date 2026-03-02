/// Hard upper bound for any listing `LIMIT`/page size to protect DB and memory usage.
pub const MAX_LISTING_ELEMENTS: i32 = 200;

/// Maximum accepted HTTP request body size for API handlers.
/// Covers JSON auth payloads and message sends while rejecting oversized bodies early.
pub const MAX_REQUEST_BODY_BYTES: usize = 64 * 1024;
