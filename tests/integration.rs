use std::time::Duration;

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

#[test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
// Create a schedule in Redis and dump it.
// Restore the dump into another key
// Both keys should have the same content
//
// Wait for a few seconds and expect duplicated items in the list
fn test_rdb_support() -> redis::RedisResult<()> {
    let client = redis::Client::open("redis://127.0.0.1:6666/")?;
    let mut con = client.get_connection()?;

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

mod incr_decr {
    use super::*;

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
    #[cfg_attr(not(feature = "integration_tests"), ignore)]
    fn test_incr_command() -> redis::RedisResult<()> {
        let client = redis::Client::open("redis://127.0.0.1:6666/")?;
        let mut con = client.get_connection()?;

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
    #[cfg_attr(not(feature = "integration_tests"), ignore)]
    fn test_decr_command() -> redis::RedisResult<()> {
        let client = redis::Client::open("redis://127.0.0.1:6666/")?;
        let mut con = client.get_connection()?;

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
}
