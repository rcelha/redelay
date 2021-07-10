use std::env::var;

pub fn get_redis_host() -> String {
    var("INTEGRATION_TEST_REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
}

pub fn get_redis_port() -> String {
    var("INTEGRATION_TEST_REDIS_PORT").unwrap_or_else(|_| "6379".to_string())
}

pub fn get_redis_connection_str() -> String {
    format!("redis://{}:{}/", get_redis_host(), get_redis_port())
}

#[cfg(feature = "test_cluster")]
pub fn open_redis_connection() -> impl redis::ConnectionLike {
    let conn = vec![get_redis_connection_str()];
    redis::cluster::ClusterClient::open(conn)
        .unwrap()
        .get_connection()
        .unwrap()
}

#[cfg(not(feature = "test_cluster"))]
pub fn open_redis_connection() -> impl redis::ConnectionLike {
    redis::Client::open(get_redis_connection_str())
        .unwrap()
        .get_connection()
        .unwrap()
}
