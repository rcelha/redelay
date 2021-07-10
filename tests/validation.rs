mod utils;
use utils::open_redis_connection;

#[test]
#[cfg_attr(not(feature = "integration_test"), ignore)]
fn test_validate_commands_with_no_delay() -> redis::RedisResult<()> {
    let mut con = open_redis_connection();
    let result: redis::RedisResult<String> = redis::cmd("SCHEDULE.ADD")
        .arg("test-schedule:{1}")
        .query(&mut con);

    assert!(result.is_err());
    Ok(())
}

#[test]
#[cfg_attr(not(feature = "integration_test"), ignore)]
fn test_validate_commands_with_command() -> redis::RedisResult<()> {
    let mut con = open_redis_connection();
    let result: redis::RedisResult<String> = redis::cmd("SCHEDULE.ADD")
        .arg("test-schedule:{1}")
        .arg(10)
        .query(&mut con);

    assert!(result.is_err());
    Ok(())
}

#[test]
#[cfg_attr(not(feature = "integration_test"), ignore)]
fn test_validate_commands_with_keyless_command() -> redis::RedisResult<()> {
    let mut con = open_redis_connection();
    let result: redis::RedisResult<String> = redis::cmd("SCHEDULE.ADD")
        .arg("test-schedule:{1}")
        .arg(10)
        .arg("PING")
        .query(&mut con);

    assert!(result.is_err());
    Ok(())
}
