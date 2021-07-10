use redis_module::{Context, NextArg, RedisError, RedisResult, RedisValue};
use std::string::String;
use std::time::{SystemTime, UNIX_EPOCH};
use std::vec::Vec;
use uuid::Uuid;

use crate::context_ext::ContextExt;

use super::{
    exec_task, open_key_and_update_timer, update_timer, ScheduleDataType, SCHEDULE_DATA_TYPE,
};

///
/// Helper function to add a task to a schedule.
/// If the schedule doesn't exist, it will create it.
///
/// Replicate a SCHEDULE.REPLICATE command to the AOF
///
fn add_task_helper_to_schedule(
    ctx: &Context,
    schedule_key: String,
    timestamp: u64,
    delayed_command: Vec<String>,
    task_id: Option<String>,
) -> Result<String, RedisError> {
    let key = ctx.open_key_writable(&schedule_key);
    let value = key.get_value::<ScheduleDataType>(&SCHEDULE_DATA_TYPE)?;

    let task_id = task_id.unwrap_or_else(|| Uuid::new_v4().to_hyphenated().to_string());

    match value {
        Some(value) => {
            value.add_task(timestamp, task_id.clone(), delayed_command.clone());
        }
        None => {
            let mut value = ScheduleDataType::new();
            value.add_task(timestamp, task_id.clone(), delayed_command.clone());
            key.set_value(&SCHEDULE_DATA_TYPE, value)?;
        }
    };

    let timestamp_str = timestamp.to_string();
    let mut replicate_args: Vec<&str> = Vec::with_capacity(3 + delayed_command.len()); // key + ts + id + [command]
    replicate_args.push(&schedule_key);
    replicate_args.push(&timestamp_str);
    replicate_args.push(&task_id);
    replicate_args.extend(delayed_command.iter().map(|x| x.as_str()));
    ctx.replicate("SCHEDULE.REPLICATE", &replicate_args);

    open_key_and_update_timer(&ctx, schedule_key, None);

    Ok(task_id)
}

///
/// SCHEDULE.REPLICATE key task_id timestamp CMD...
///
pub fn replicate(ctx: &Context, args: Vec<String>) -> RedisResult {
    ctx.log_notice(format!("{:?}", args).as_str());

    let mut args = args.into_iter().skip(1);
    let schedule_key = args.next_string()?;
    let timestamp = args.next_u64()?;
    let task_id = args.next_string()?;
    let delayed_command: Vec<String> = args.collect();

    let command_keys = ctx.get_command_keys(&delayed_command)?;
    if ctx.is_keys_position_request() {
        let offset = 4; // (0)SCHEDULE.REPLICATE (1)KEY (2)task_id (3)DELAY [CMD] ==
        ctx.key_at_pos(1);
        for key_pos in command_keys {
            ctx.key_at_pos(offset + key_pos);
        }
        return Ok(RedisValue::NoReply);
    }

    let task_id =
        add_task_helper_to_schedule(ctx, schedule_key, timestamp, delayed_command, Some(task_id))?;
    Ok(RedisValue::BulkString(task_id))
}

///
/// SCHEDULE.ADD key delay CMD...
///
pub fn add(ctx: &Context, args: Vec<String>) -> RedisResult {
    ctx.log_notice(format!("{:?}", args).as_str());
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?;

    let mut args = args.into_iter().skip(1);
    let schedule_key = args.next_string()?;
    let delay = args.next_u64()?;
    let timestamp = now.as_secs() + delay;
    let delayed_command: Vec<String> = args.collect();

    let command_keys = ctx.get_command_keys(&delayed_command)?;
    if ctx.is_keys_position_request() {
        let offset = 3; // (0)SCHEDULE.ADD (1)KEY (2)DELAY [CMD] ==
        ctx.key_at_pos(1);
        for key_pos in command_keys {
            ctx.key_at_pos(offset + key_pos);
        }
        return Ok(RedisValue::NoReply);
    }

    let task_id = add_task_helper_to_schedule(ctx, schedule_key, timestamp, delayed_command, None)?;
    Ok(RedisValue::BulkString(task_id))
}

///
/// SCHEDULE.REM key task-id
///
pub fn rem(ctx: &Context, args: Vec<String>) -> RedisResult {
    ctx.log_notice(format!("{:?}", args).as_str());

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
/// Helper function to execute task from a schedule,
///
/// This function will propagate two items into the AOF:
///     1. SCHEDULE.REM (remove this task from the schedule)
///     2. [COMMAND] (the command this task executed)
///
/// Important: This function will not create/update timers, this
/// is something that must be handled by the caller
///
fn execute_schedule_task(ctx: &Context, schedule_key: String, task_id: String) -> RedisResult {
    let key = ctx.open_key_writable(&schedule_key);

    if let Some(value) = key.get_value::<ScheduleDataType>(&SCHEDULE_DATA_TYPE)? {
        let task = value.del_task(task_id.clone());
        if let Some(task) = task {
            exec_task(ctx, &task.args).map_err(|error| {
                let msg = format!(
                    "Failed to execute task (key={}, id={}, task={:?}); Error={:#?}",
                    schedule_key, task_id, task, error
                );
                ctx.log_warning(&msg);
                error
            })?;

            let mut args_iter = task.args.iter();
            let cmd = args_iter.next().unwrap().as_str();
            let args: Vec<&str> = args_iter.map(|x| x.as_str()).collect();
            ctx.replicate("SCHEDULE.REM", &[&schedule_key, &task_id]);
            ctx.replicate(cmd, &args);
        }
        Ok(RedisValue::Null)
    } else {
        Ok(RedisValue::Null)
    }
}

///
/// SCHEDULE.EXEC key task-id
///
pub fn exec(ctx: &Context, args: Vec<String>) -> RedisResult {
    ctx.log_notice(format!("{:?}", args).as_str());

    let mut args = args.iter().skip(1);
    let schedule_key = args.next_string()?;
    let task_id = args.next_string()?;

    execute_schedule_task(ctx, schedule_key.clone(), task_id)?;
    open_key_and_update_timer(&ctx, schedule_key, None);

    Ok(RedisValue::Null)
}

///
/// SCHEDULE.EXECDUE key timestamp
///
pub fn exec_due(ctx: &Context, args: Vec<String>) -> RedisResult {
    ctx.log_notice(format!("{:?}", args).as_str());

    let mut args = args.iter().skip(1);
    let schedule_key = args.next_string()?;
    let timestamp = args.next_u64()?;

    let key = ctx.open_key(&schedule_key); // Open as read, SCHEDULE.EXEC will open to write
    let value = key.get_value::<ScheduleDataType>(&SCHEDULE_DATA_TYPE);
    if let Ok(Some(value)) = value {
        for (task_timestamp, task_id) in value.timetable_iter() {
            if *task_timestamp > timestamp {
                break;
            }
            execute_schedule_task(ctx, schedule_key.clone(), task_id.to_string())?;
        }
    }

    open_key_and_update_timer(&ctx, schedule_key, None);
    Ok(RedisValue::Null)
}

///
/// SCHEDULE.SCAN key
///
pub fn scan(ctx: &Context, args: Vec<String>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let key = args.next_string()?;
    let key = ctx.open_key(&key);

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

///
/// SCHEDULE.INCRBY KEY TASK-ID SECONDS
///
pub fn incrby(ctx: &Context, args: Vec<String>) -> RedisResult {
    ctx.log_notice(format!("{:?}", args).as_str());
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?;

    let mut args = args.into_iter().skip(1);
    let schedule_key = args.next_string()?;
    let task_id = args.next_string()?;
    let inc_size = args.next_u64()?;
    let key = ctx.open_key_writable(&schedule_key);

    match key.get_value::<ScheduleDataType>(&SCHEDULE_DATA_TYPE)? {
        Some(value) => {
            let new_timestamp = value.incr(task_id, inc_size);
            ctx.replicate_verbatim();
            match new_timestamp {
                Some(new_timestamp) => {
                    update_timer(&ctx, schedule_key, value, now);
                    Ok(RedisValue::BulkString(new_timestamp.to_string()))
                }
                _ => Ok(RedisValue::Null),
            }
        }
        None => Ok(RedisValue::Null),
    }
}

///
/// SCHEDULE.DECRBY KEY TASK-ID SECONDS
///
pub fn decrby(ctx: &Context, args: Vec<String>) -> RedisResult {
    ctx.log_notice(format!("{:?}", args).as_str());
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?;

    let mut args = args.into_iter().skip(1);
    let schedule_key = args.next_string()?;
    let task_id = args.next_string()?;
    let inc_size = args.next_u64()?;
    let key = ctx.open_key_writable(&schedule_key);

    match key.get_value::<ScheduleDataType>(&SCHEDULE_DATA_TYPE)? {
        Some(value) => {
            let new_timestamp = value.decr(task_id, inc_size);
            ctx.replicate_verbatim();
            match new_timestamp {
                Some(new_timestamp) => {
                    update_timer(&ctx, schedule_key, value, now);
                    Ok(RedisValue::BulkString(new_timestamp.to_string()))
                }
                _ => Ok(RedisValue::Null),
            }
        }
        None => Ok(RedisValue::Null),
    }
}
