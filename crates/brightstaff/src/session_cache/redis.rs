use std::time::Duration;

use async_trait::async_trait;
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use redis::AsyncCommands;

use super::{SessionBinding, SessionCache, SessionCacheError};

const KEY_PREFIX: &str = "plano:affinity:";

/// Session affinity is an optimization, so a slow backend must never hold up a routing
/// decision. Reconnects happen in the background between these attempts.
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(2);

pub struct RedisSessionCache {
    /// A `ConnectionManager` rather than a bare `MultiplexedConnection`: the latter never
    /// reconnects, so one dropped socket (idle timeout, failover, network blip) disables
    /// session affinity for the rest of the process's lifetime.
    conn: ConnectionManager,
}

impl RedisSessionCache {
    pub async fn new(url: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(url)?;
        let config = ConnectionManagerConfig::new()
            .set_connection_timeout(CONNECTION_TIMEOUT)
            .set_response_timeout(RESPONSE_TIMEOUT);
        let conn = ConnectionManager::new_with_config(client, config).await?;
        Ok(Self { conn })
    }

    fn make_key(key: &str) -> String {
        format!("{KEY_PREFIX}{key}")
    }
}

#[async_trait]
impl SessionCache for RedisSessionCache {
    async fn get(&self, key: &str) -> Result<Option<SessionBinding>, SessionCacheError> {
        let mut conn = self.conn.clone();
        let value: Option<String> = conn
            .get(Self::make_key(key))
            .await
            .map_err(SessionCacheError::new)?;
        // A binding written by an incompatible build is unusable but is not a backend
        // failure: treat it as absent and let the session re-bind.
        Ok(value.and_then(|v| serde_json::from_str(&v).ok()))
    }

    async fn put(
        &self,
        key: &str,
        binding: SessionBinding,
        ttl: Duration,
    ) -> Result<(), SessionCacheError> {
        let mut conn = self.conn.clone();
        // The Redis TTL is only a GC bound; warmth is decided by the router from
        // `binding.last_used`, not by expiry here.
        let ttl_secs = ttl.as_secs().max(1);
        let json = serde_json::to_string(&binding).map_err(SessionCacheError::new)?;
        conn.set_ex::<_, _, ()>(Self::make_key(key), json, ttl_secs)
            .await
            .map_err(SessionCacheError::new)
    }

    async fn remove(&self, key: &str) -> Result<(), SessionCacheError> {
        let mut conn = self.conn.clone();
        conn.del::<_, ()>(Self::make_key(key))
            .await
            .map_err(SessionCacheError::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding() -> SessionBinding {
        SessionBinding {
            anchor_model: "gpt-4o".to_string(),
            default_model: "gpt-4o".to_string(),
            requested_model: "gpt-4o".to_string(),
            route_name: None,
            prefix_hash: None,
            last_used: std::time::SystemTime::now(),
            cached_tokens: 0,
            baseline_usd: 0.0,
            switch_spend_usd: 0.0,
            switches: 0,
            session_cost_usd: 0.0,
            history: Vec::new(),
        }
    }

    /// A non-reconnecting connection turned one dropped socket into permanently disabled
    /// affinity, and every later lookup was indistinguishable from a cold session.
    ///
    /// Needs a disposable Redis:
    /// `PLANO_TEST_REDIS_URL=redis://127.0.0.1:6379 cargo test -p brightstaff \
    ///  session_cache::redis -- --ignored`
    #[tokio::test]
    #[ignore = "requires a Redis at PLANO_TEST_REDIS_URL"]
    async fn recovers_after_the_server_drops_the_connection() {
        let url = std::env::var("PLANO_TEST_REDIS_URL").expect("PLANO_TEST_REDIS_URL must be set");
        let cache = RedisSessionCache::new(&url).await.expect("connect");
        let key = "reconnect-test";

        cache
            .put(key, binding(), Duration::from_secs(60))
            .await
            .expect("initial put");
        assert!(cache.get(key).await.expect("initial get").is_some());

        // Drop the cache's connection server-side, standing in for an idle timeout or a
        // failover. CLIENT KILL skips the caller by default, so this connection survives.
        let client = redis::Client::open(url.as_str()).expect("client");
        let mut killer = client
            .get_multiplexed_async_connection()
            .await
            .expect("killer connection");
        let killed: i64 = redis::cmd("CLIENT")
            .arg("KILL")
            .arg("TYPE")
            .arg("normal")
            .query_async(&mut killer)
            .await
            .expect("client kill");
        assert!(killed > 0, "expected to kill at least the cache connection");

        // Reconnection happens in the background, so the first call after the kill may
        // still hit the dead socket.
        let mut last_err = None;
        for _ in 0..20 {
            match cache.get(key).await {
                Ok(found) => {
                    assert!(found.is_some(), "binding lost across reconnect");
                    return;
                }
                Err(err) => {
                    last_err = Some(err.to_string());
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        }
        panic!("cache never recovered after connection drop: {last_err:?}");
    }
}
