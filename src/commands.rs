use redis_module::raw;
use redis_module::{Context, NextArg, RedisError, RedisResult, RedisValue};
use std::string::String;
use std::time::{SystemTime, UNIX_EPOCH};
use std::vec::Vec;
use uuid::Uuid;

use super::{exec_task, update_timer, ScheduleDataType, SCHEDULE_DATA_TYPE};

fn add_task_helper(
    ctx: &Context,
    schedule_key: String,
    timestamp: u64,
    delayed_command: Vec<String>,
    task_id: Option<String>, // TODO remove this hack?
) -> Result<String, RedisError> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let key = ctx.open_key_writable(&schedule_key);
    let value = key.get_value::<ScheduleDataType>(&SCHEDULE_DATA_TYPE)?;

    // If this is a brand new task, we will create/update a timer
    let update_timer_ = task_id.is_none();
    let task_id = task_id.unwrap_or_else(|| Uuid::new_v4().to_hyphenated().to_string());

    match value {
        Some(value) => {
            value.add_task(timestamp, task_id.clone(), delayed_command.clone());
            if update_timer_ {
                update_timer(&ctx, schedule_key.clone(), value, now);
            }
        }
        None => {
            let mut value = ScheduleDataType::new();
            value.add_task(timestamp, task_id.clone(), delayed_command.clone());
            if update_timer_ {
                update_timer(&ctx, schedule_key.clone(), &mut value, now);
            }
            key.set_value(&SCHEDULE_DATA_TYPE, value)?;
        }
    };

    let timestamp_str = timestamp.to_string();
    let mut replicate_args: Vec<&str> = Vec::with_capacity(delayed_command.len() + 3); // key + ts + id + commd
    replicate_args.push(schedule_key.as_str());
    replicate_args.push(timestamp_str.as_str());
    replicate_args.push(task_id.as_str());
    replicate_args.extend(delayed_command.iter().map(|x| x.as_str()));
    raw::replicate(ctx.get_raw(), "SCHEDULE.REPLICATE", &&replicate_args);

    Ok(task_id)
}

///
/// SCHEDULE.REPLICATE key task_id timestamp CMD...
///
pub fn replicate(ctx: &Context, args: Vec<String>) -> RedisResult {
    ctx.log_notice(format!("[schedule.replicate]: {:?}", args).as_str());

    let mut args = args.into_iter().skip(1);
    let schedule_key = args.next_string()?;
    let timestamp = args.next_u64()?;
    let task_id = args.next_string()?;
    let delayed_command: Vec<String> = args.collect();

    let task_id = add_task_helper(ctx, schedule_key, timestamp, delayed_command, Some(task_id))?;
    Ok(RedisValue::BulkString(task_id))
}

///
/// SCHEDULE.ADD key delay CMD...
///
pub fn add(ctx: &Context, args: Vec<String>) -> RedisResult {
    ctx.log_notice(format!("[schedule.add]: {:?}", args).as_str());
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?;

    let mut args = args.into_iter().skip(1);
    let schedule_key = args.next_string()?;
    let delay = args.next_u64()?;
    let timestamp = now.as_secs() + delay;
    let delayed_command: Vec<String> = args.collect();

    let task_id = add_task_helper(ctx, schedule_key.clone(), timestamp, delayed_command, None)?;
    Ok(RedisValue::BulkString(task_id))
}

///
/// SCHEDULE.REM key task-id
///
pub fn rem(ctx: &Context, args: Vec<String>) -> RedisResult {
    ctx.log_notice(format!("[schedule.rem]: {:?}", args).as_str());

    let mut args = args.into_iter().skip(1);
    let key = args.next_string()?;
    let task_id = args.next_string()?;
    let key = ctx.open_key_writable(&key);

    match key.get_value::<ScheduleDataType>(&SCHEDULE_DATA_TYPE)? {
        Some(value) => {
            value.del_task(task_id);
            ctx.replicate_verbatim();
            Ok(RedisValue::Null)
        }
        None => Ok(RedisValue::Null),
    }
}

///
/// SCHEDULE.EXEC key task-id
///
pub fn exec(ctx: &Context, args: Vec<String>) -> RedisResult {
    ctx.log_notice(format!("[schedule.exec]: {:?}", args).as_str());

    let mut args = args.iter().skip(1);
    let key = args.next_string()?;
    let task_id = args.next_string()?;

    let key = ctx.open_key_writable(&key);
    match key.get_value::<ScheduleDataType>(&SCHEDULE_DATA_TYPE)? {
        Some(value) => {
            let task = value.del_task(task_id);
            if let Some(task) = task {
                exec_task(ctx, &task.args)?;
            }
            ctx.replicate_verbatim();
            Ok(RedisValue::Null)
        }
        None => Ok(RedisValue::Null),
    }
}

///
/// SCHEDULE.SCAN key
///
pub fn scan(ctx: &Context, args: Vec<String>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let key = args.next_string()?;
    let key = ctx.open_key_writable(&key);

    match key.get_value::<ScheduleDataType>(&SCHEDULE_DATA_TYPE)? {
        Some(value) => {
            let ret: Vec<RedisValue> = value
                .to_vec()
                .drain(..)
                .map(|(timestamp, task_id, args)| {
                    RedisValue::Array(vec![
                        RedisValue::from(timestamp.to_string()),
                        RedisValue::from(task_id),
                        RedisValue::from(args),
                    ])
                })
                .collect();
            Ok(RedisValue::Array(ret))
        }
        None => Ok(RedisValue::Null),
    }
}
