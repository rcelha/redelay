use std::time::Duration;

mod utils;
use utils::open_redis_connection;

fn cleanup(con: &mut dyn redis::ConnectionLike, prefix: &str) {
    redis::pipe()
        .cmd("DEL")
        .arg(prefix.to_owned() + "-schedule")
        .arg(prefix.to_owned() + "-list")
        .execute(con)
}

fn create_table(
    con: &mut dyn redis::ConnectionLike,
    prefix: &str,
    delay: usize,
) -> redis::RedisResult<(String, String)> {
    redis::pipe()
        .cmd("SCHEDULE.ADD")
        .arg(prefix.to_owned() + "-schedule")
        .arg(delay)
        .arg("rpush")
        .arg(prefix.to_owned() + "-list")
        .arg("item-1")
        .cmd("SCHEDULE.ADD")
        .arg(prefix.to_owned() + "-schedule")
        .arg(delay)
        .arg("rpush")
        .arg(prefix.to_owned() + "-list")
        .arg("item-2")
        .query(con)
}

#[test]
#[cfg_attr(not(feature = "integration_test"), ignore)]
#[cfg_attr(feature = "test_cluster", ignore)]
fn test_incr_command() -> redis::RedisResult<()> {
    let mut con = open_redis_connection();

    cleanup(&mut con, "test-incr");
    let (task1, task2) = create_table(&mut con, "test-incr", 2)?;

    redis::pipe()
        .cmd("SCHEDULE.INCRBY")
        .arg("test-incr-schedule")
        .arg(&task1)
        .arg(6)
        .cmd("SCHEDULE.INCRBY")
        .arg("test-incr-schedule")
        .arg(&task2)
        .arg(6)
        .execute(&mut con);

    std::thread::sleep(Duration::from_secs(3));
    let list_size: usize = redis::cmd("LLEN").arg("test-incr-list").query(&mut con)?;
    assert_eq!(list_size, 0);

    std::thread::sleep(Duration::from_secs(5));
    let list_size: usize = redis::cmd("LLEN").arg("test-incr-list").query(&mut con)?;
    assert_eq!(list_size, 2);
    Ok(())
}

#[test]
#[cfg_attr(not(feature = "integration_test"), ignore)]
#[cfg_attr(feature = "test_cluster", ignore)]
fn test_decr_command() -> redis::RedisResult<()> {
    let mut con = open_redis_connection();

    cleanup(&mut con, "test-decr");
    let (task1, task2) = create_table(&mut con, "test-decr", 600)?;

    redis::pipe()
        .cmd("SCHEDULE.DECRBY")
        .arg("test-decr-schedule")
        .arg(&task1)
        .arg(599)
        .cmd("SCHEDULE.DECRBY")
        .arg("test-decr-schedule")
        .arg(&task2)
        .arg(599)
        .execute(&mut con);

    std::thread::sleep(Duration::from_secs(5));
    let list_size: usize = redis::cmd("LLEN").arg("test-decr-list").query(&mut con)?;
    assert_eq!(list_size, 2);
    Ok(())
}
