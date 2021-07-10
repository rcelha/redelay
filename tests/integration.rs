use std::time::Duration;

mod utils;
use utils::open_redis_connection;

#[test]
#[cfg_attr(not(feature = "integration_test"), ignore)]
#[cfg_attr(feature = "test_cluster", ignore)]
// Creates a schedule that inserts a list into a list every second.
// Wait as much time as needed to get the items in the list, and
// pop item by item to test if they were inserted in the right
// order
fn test_populating_fifo() -> redis::RedisResult<()> {
    let mut con = open_redis_connection();

    redis::pipe()
        .cmd("DEL")
        .arg("test-schedule")
        .arg("test-fifo")
        .query(&mut con)?;

    redis::pipe()
        // delay item-3
        .cmd("SCHEDULE.ADD")
        .arg("test-schedule")
        .arg(3)
        .arg("rpush")
        .arg("test-fifo")
        .arg("item-3")
        // delay item-2
        .cmd("SCHEDULE.ADD")
        .arg("test-schedule")
        .arg(2)
        .arg("rpush")
        .arg("test-fifo")
        .arg("item-2")
        // delay item-1
        .cmd("SCHEDULE.ADD")
        .arg("test-schedule")
        .arg(1)
        .arg("rpush")
        .arg("test-fifo")
        .arg("item-1")
        .query(&mut con)?;

    let schedule: Vec<Vec<String>> = redis::cmd("SCHEDULE.SCAN")
        .arg("test-schedule")
        .query(&mut con)?;
    assert_eq!(schedule.len(), 3);

    // wait a few second and test test-fifo
    std::thread::sleep(Duration::from_secs(4));
    let item: Option<String> = redis::cmd("LPOP").arg("test-fifo").query(&mut con)?;
    assert_eq!(item, Some("item-1".to_string()));

    let item: Option<String> = redis::cmd("LPOP").arg("test-fifo").query(&mut con)?;
    assert_eq!(item, Some("item-2".to_string()));

    let item: Option<String> = redis::cmd("LPOP").arg("test-fifo").query(&mut con)?;
    assert_eq!(item, Some("item-3".to_string()));

    let schedule: Vec<Vec<String>> = redis::cmd("SCHEDULE.SCAN")
        .arg("test-schedule")
        .query(&mut con)?;
    assert_eq!(schedule.len(), 0);

    Ok(())
}

#[test]
#[cfg_attr(not(feature = "integration_test"), ignore)]
#[cfg_attr(feature = "test_cluster", ignore)]
// Create a schedule in Redis and dump it.
// Restore the dump into another key
// Both keys should have the same content
//
// Wait for a few seconds and expect duplicated items in the list
fn test_rdb_support() -> redis::RedisResult<()> {
    let mut con = open_redis_connection();

    redis::pipe()
        .cmd("DEL")
        .arg("test-rdb-orig")
        .arg("test-rdb-dest")
        .arg("test-rdb-list")
        .query(&mut con)?;

    redis::pipe()
        // delay item-1
        .cmd("SCHEDULE.ADD")
        .arg("test-rdb-orig")
        .arg(3)
        .arg("rpush")
        .arg("test-rdb-list")
        .arg("item-1")
        // delay item-2
        .cmd("SCHEDULE.ADD")
        .arg("test-rdb-orig")
        .arg(4)
        .arg("rpush")
        .arg("test-rdb-list")
        .arg("item-2")
        .query(&mut con)?;

    let orig: Vec<u8> = redis::cmd("DUMP").arg("test-rdb-orig").query(&mut con)?;

    redis::cmd("RESTORE")
        .arg("test-rdb-dest")
        .arg(0)
        .arg(orig.as_slice())
        .query(&mut con)?;

    let orig: Vec<Vec<String>> = redis::cmd("SCHEDULE.SCAN")
        .arg("test-rdb-orig")
        .query(&mut con)?;

    let dest: Vec<Vec<String>> = redis::cmd("SCHEDULE.SCAN")
        .arg("test-rdb-dest")
        .query(&mut con)?;

    assert_eq!(orig, dest);

    std::thread::sleep(Duration::from_secs(4));
    let final_list: Vec<String> = redis::cmd("LRANGE")
        .arg("test-rdb-list")
        .arg(0)
        .arg(-1)
        .query(&mut con)?;

    assert_eq!(final_list.len(), 4);
    assert_eq!(final_list.iter().filter(|x| *x == "item-1").count(), 2);
    assert_eq!(final_list.iter().filter(|x| *x == "item-2").count(), 2);

    Ok(())
}
