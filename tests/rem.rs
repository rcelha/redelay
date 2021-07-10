use std::time::Duration;

mod utils;
use utils::open_redis_connection;

const PREFIX: &str = "{test-rem}:";

fn k(val: &str) -> String {
    PREFIX.to_string() + val
}

fn cleanup() {
    let mut con = open_redis_connection();
    redis::cmd("DEL")
        .arg(k("schedule"))
        .arg(k("list"))
        .execute(&mut con);
}

fn setup(delay: usize) -> Vec<String> {
    let mut con = open_redis_connection();
    vec![
        redis::cmd("SCHEDULE.ADD")
            .arg(k("schedule"))
            .arg(delay)
            .arg("rpush")
            .arg(k("list"))
            .arg("item-1")
            .query(&mut con)
            .unwrap(),
        redis::cmd("SCHEDULE.ADD")
            .arg(k("schedule"))
            .arg(delay)
            .arg("rpush")
            .arg(k("list"))
            .arg("item-2")
            .query(&mut con)
            .unwrap(),
    ]
}

#[test]
#[cfg_attr(not(feature = "integration_test"), ignore)]
fn test_schedule_and_remove() -> redis::RedisResult<()> {
    cleanup();
    let tasks = setup(3);
    assert!(tasks.len() > 0);

    let mut con = open_redis_connection();
    for task_id in tasks {
        let _: () = redis::cmd("SCHEDULE.REM")
            .arg(k("schedule"))
            .arg(task_id)
            .query(&mut con)
            .unwrap();
    }

    let schedule: Vec<Vec<String>> = redis::cmd("SCHEDULE.SCAN")
        .arg(k("schedule"))
        .query(&mut con)?;
    assert_eq!(schedule.len(), 0);

    std::thread::sleep(Duration::from_secs(4));

    let list: Vec<String> = redis::cmd("LRANGE")
        .arg(k("list"))
        .arg(0)
        .arg(-1)
        .query(&mut con)?;
    assert_eq!(list.len(), 0);

    Ok(())
}
