use std::time::Duration;

use redis;

#[test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
// Creates a schedule that inserts a list into a list every second.
// Wait as much time as needed to get the items in the list, and
// pop item by item to test if they were inserted in the right
// order
fn test_populating_fifo() -> redis::RedisResult<()> {
    let client = redis::Client::open("redis://127.0.0.1:6666/")?;
    let mut con = client.get_connection()?;

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
    let item: String = redis::cmd("LPOP").arg("test-fifo").query(&mut con)?;
    assert_eq!(&item, "item-1");

    let item: String = redis::cmd("LPOP").arg("test-fifo").query(&mut con)?;
    assert_eq!(&item, "item-2");

    let item: String = redis::cmd("LPOP").arg("test-fifo").query(&mut con)?;
    assert_eq!(&item, "item-3");

    let schedule: Vec<Vec<String>> = redis::cmd("SCHEDULE.SCAN")
        .arg("test-schedule")
        .query(&mut con)?;
    assert_eq!(schedule.len(), 0);

    Ok(())
}
