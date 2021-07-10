mod utils;
use utils::open_redis_connection;

#[test]
#[cfg_attr(not(feature = "integration_test"), ignore)]
fn test_cluster_same_slot() -> redis::RedisResult<()> {
    let mut con = open_redis_connection();

    redis::cmd("DEL")
        .arg("test-schedule:{1}")
        .arg("test-fifo:{1}")
        .query(&mut con)?;

    let task_id: redis::RedisResult<String> = redis::cmd("SCHEDULE.ADD")
        .arg("test-schedule:{1}")
        .arg(3)
        .arg("rpush")
        .arg("test-fifo:{1}")
        .arg("item-3")
        .query(&mut con);

    assert!(task_id.is_ok());

    Ok(())
}

#[test]
#[cfg_attr(not(feature = "integration_test"), ignore)]
fn test_cluster_same_slot_2() -> redis::RedisResult<()> {
    let mut con = open_redis_connection();

    redis::cmd("DEL")
        .arg("test-schedule:{3}")
        .arg("test-fifo:{3}")
        .query(&mut con)?;

    let task_id: redis::RedisResult<String> = redis::cmd("SCHEDULE.ADD")
        .arg("test-schedule:{3}")
        .arg(3)
        .arg("rpush")
        .arg("test-fifo:{3}")
        .arg("item-3")
        .query(&mut con);

    assert!(task_id.is_ok());

    Ok(())
}

#[test]
#[cfg_attr(not(feature = "integration_test"), ignore)]
fn test_cluster_different_slots() -> redis::RedisResult<()> {
    let mut con = open_redis_connection();

    redis::cmd("DEL")
        .arg("test-schedule-diff:{1}")
        .query(&mut con)?;
    redis::cmd("DEL")
        .arg("test-fifo-diff:{3}")
        .query(&mut con)?;

    let task_id: redis::RedisResult<String> = redis::cmd("SCHEDULE.ADD")
        .arg("test-schedule-diff:{1}")
        .arg(3)
        .arg("rpush")
        .arg("test-fifo-diff:{3}")
        .arg("item-3")
        .query(&mut con);

    if cfg!(feature = "test_cluster") {
        assert!(task_id.is_err());
        let task_err = task_id.unwrap_err();
        assert_eq!(task_err.kind(), redis::ErrorKind::CrossSlot);
    } else {
        assert!(task_id.is_ok());
    }

    Ok(())
}
