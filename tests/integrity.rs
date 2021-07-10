use std::time::Duration;

mod utils;
use utils::open_redis_connection;

#[test]
#[cfg_attr(not(feature = "integrity_test_setup"), ignore)]
// This is not a test per-se, but it sets up data into redis
// Once this is done, we can stop redis and let it restore its
// data (from either the AOF or the RDB) and run the data integrity
// checks
//
// The tests in this file are suppose to run on both standalone
// and cluster mode
//
// That is the schedule:
//      - 3s -> rpush item-1
//      - 4s -> rpush item-2
//      - 5s -> rpush item-3
//      - 60s -> rpush item-4
fn setup_integrity_test_data() -> redis::RedisResult<()> {
    let mut con = open_redis_connection();

    let _ = redis::cmd("DEL")
        .arg("{integrity}-schedule")
        .arg("{integrity}-fifo")
        .query(&mut con)?;

    redis::cmd("SCHEDULE.ADD")
        .arg("{integrity}-schedule")
        .arg(5)
        .arg("rpush")
        .arg("{integrity}-fifo")
        .arg("item-3")
        .query(&mut con)?;

    redis::cmd("SCHEDULE.ADD")
        .arg("{integrity}-schedule")
        .arg(4)
        .arg("rpush")
        .arg("{integrity}-fifo")
        .arg("item-2")
        .query(&mut con)?;

    redis::cmd("SCHEDULE.ADD")
        .arg("{integrity}-schedule")
        .arg(3)
        .arg("rpush")
        .arg("{integrity}-fifo")
        .arg("item-1")
        .query(&mut con)?;

    redis::cmd("SCHEDULE.ADD")
        .arg("{integrity}-schedule")
        .arg(60)
        .arg("rpush")
        .arg("{integrity}-fifo")
        .arg("item-4")
        .query(&mut con)?;

    std::thread::sleep(Duration::from_secs(5));
    check_first_batch(&mut con)?;

    Ok(())
}

#[test]
#[cfg_attr(not(feature = "integrity_test"), ignore)]
fn integrity_check() -> redis::RedisResult<()> {
    let mut con = open_redis_connection();
    check_first_batch(&mut con)?;
    std::thread::sleep(Duration::from_secs(60));
    check_second_batch(&mut con)?;
    Ok(())
}

fn check_first_batch(con: &mut dyn redis::ConnectionLike) -> redis::RedisResult<()> {
    let fifo: Vec<String> = redis::cmd("LRANGE")
        .arg("{integrity}-fifo")
        .arg(0)
        .arg(-1)
        .query(con)?;
    assert_eq!(fifo.len(), 3);
    Ok(())
}

fn check_second_batch(con: &mut dyn redis::ConnectionLike) -> redis::RedisResult<()> {
    let fifo: Vec<String> = redis::cmd("LRANGE")
        .arg("{integrity}-fifo")
        .arg(0)
        .arg(-1)
        .query(con)?;
    assert_eq!(fifo.len(), 4);
    Ok(())
}
