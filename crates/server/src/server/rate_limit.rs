use std::fmt::Debug;
use std::num::NonZeroU32;

use dashmap::DashSet;
use governor::clock::DefaultClock;
use governor::state::keyed::DashMapStateStore;
use governor::{Quota, RateLimiter as GovernorRateLimiter};
use tracing::warn;

use crate::error::RequestError;
use crate::models::session::SessionId;
use crate::models::user::UserId;

type KeyedRateLimiter<K> = GovernorRateLimiter<K, DashMapStateStore<K>, DefaultClock>;

pub struct RateLimiter {
    login_by_alias: KeyedRateLimiter<String>,
    refresh_by_session: KeyedRateLimiter<SessionId>,
    change_password_by_user: KeyedRateLimiter<UserId>,
    login_limited_keys: DashSet<String>,
    refresh_limited_keys: DashSet<SessionId>,
    change_password_limited_keys: DashSet<UserId>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::new_with_quotas(
            quota_per_minute(6),
            quota_per_minute(30),
            quota_per_minute(5),
        )
    }

    fn new_with_quotas(
        login_quota: Quota,
        refresh_quota: Quota,
        change_password_quota: Quota,
    ) -> Self {
        Self {
            login_by_alias: KeyedRateLimiter::keyed(login_quota),
            refresh_by_session: KeyedRateLimiter::keyed(refresh_quota),
            change_password_by_user: KeyedRateLimiter::keyed(change_password_quota),
            login_limited_keys: DashSet::new(),
            refresh_limited_keys: DashSet::new(),
            change_password_limited_keys: DashSet::new(),
        }
    }

    pub fn check_login_alias(&self, alias: &str) -> Result<(), RequestError> {
        check_key_with_log_once(
            &self.login_by_alias,
            &self.login_limited_keys,
            alias.to_string(),
            "auth/login",
        )
    }

    pub fn check_refresh_session(&self, session_id: SessionId) -> Result<(), RequestError> {
        check_key_with_log_once(
            &self.refresh_by_session,
            &self.refresh_limited_keys,
            session_id,
            "auth/refresh",
        )
    }

    pub fn check_change_password_user(&self, user_id: UserId) -> Result<(), RequestError> {
        check_key_with_log_once(
            &self.change_password_by_user,
            &self.change_password_limited_keys,
            user_id,
            "auth/change-password",
        )
    }
}

fn check_key_with_log_once<K: Clone + Eq + std::hash::Hash + Debug>(
    limiter: &KeyedRateLimiter<K>,
    limited_keys: &DashSet<K>,
    key: K,
    subject: &'static str,
) -> Result<(), RequestError> {
    match limiter.check_key(&key) {
        Ok(()) => {
            limited_keys.remove(&key);
            Ok(())
        }
        Err(_) => {
            if limited_keys.insert(key.clone()) {
                warn!(subject, key = ?key, "rate limit exceeded");
            }
            Err(RequestError::RateLimited(subject))
        }
    }
}

fn quota_per_minute(max_requests: u32) -> Quota {
    let max_requests = NonZeroU32::new(max_requests).expect("rate limit must be non-zero");
    Quota::per_minute(max_requests)
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::*;

    #[test]
    fn blocks_when_limit_is_reached() {
        let limiter = RateLimiter::new_with_quotas(
            Quota::per_second(NonZeroU32::new(2).unwrap()),
            Quota::per_second(NonZeroU32::new(2).unwrap()),
            Quota::per_second(NonZeroU32::new(2).unwrap()),
        );

        assert!(limiter.check_login_alias("alice").is_ok());
        assert!(limiter.check_login_alias("alice").is_ok());
        assert!(matches!(
            limiter.check_login_alias("alice"),
            Err(RequestError::RateLimited("auth/login"))
        ));
    }

    #[test]
    fn keeps_limits_independent_per_key() {
        let limiter = RateLimiter::new_with_quotas(
            Quota::per_second(NonZeroU32::new(1).unwrap()),
            Quota::per_second(NonZeroU32::new(1).unwrap()),
            Quota::per_second(NonZeroU32::new(1).unwrap()),
        );

        assert!(limiter.check_login_alias("alice").is_ok());
        assert!(limiter.check_login_alias("bob").is_ok());
        assert!(matches!(
            limiter.check_login_alias("alice"),
            Err(RequestError::RateLimited("auth/login"))
        ));
    }
}
