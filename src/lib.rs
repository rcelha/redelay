#[macro_use]
extern crate redis_module;
use redis_module::{raw, Context, RedisResult};
use std::string::String;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::vec::Vec;

mod data_types;
use data_types::*;
mod commands;
pub mod skiplist_ext;

// Extracts the embeded commands from a task and execute it
//
// This function returns the command's result
fn exec_task(ctx: &Context, task: &[String]) -> RedisResult {
    let (command_slice, args_slice) = task.split_at(1);
    let command = command_slice[0].as_str();
    let args: Vec<&str> = args_slice.iter().map(|x| x.as_str()).collect();
    ctx.call(command, &args)
}

// Execute the due tasks and schedule the next execution
fn exec_due_tasks(ctx: &Context, schedule_key: String) {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let now_timestamp = now.as_secs();

    let key = ctx.open_key_writable(&schedule_key);
    let value = key.get_value::<ScheduleDataType>(&SCHEDULE_DATA_TYPE);

    if let Ok(Some(value)) = value {
        while let Some((_timestamp, task_id, task)) = value.pop_by_timestamp(now_timestamp) {
            // TODO is this the right place?
            raw::replicate(
                ctx.get_raw(),
                "SCHEDULE.EXEC",
                &[&schedule_key, task_id.as_str()],
            );
            // TODO Move error into dead letter
            if let Err(err) = exec_task(&ctx, &task) {
                let msg = format!(
                    "Failed to exec task (key={}, id={}, task={:?}); Error={}",
                    schedule_key, task_id, task, err
                );
                ctx.log_warning(&msg);
            }
        }
        value.timer_id = None;

        // If the schedule has more items, schedule the next
        if value.len() > 0 {
            update_timer(ctx, schedule_key, value, now);
        }
    }
}

// Updates a schedules's timer
//
// If there is no next item, ignore the operation
// If the current timer is later then the head, stop current timer
fn update_timer(
    ctx: &Context,
    schedule_key: String,
    schedule: &mut ScheduleDataType,
    now: Duration,
) {
    let next_timestamp = match schedule.get_min_timestamp() {
        Some(v) => v,
        _ => return,
    };
    let next_duration = Duration::from_secs(next_timestamp);
    let next_duration = if next_duration < now {
        Duration::from_secs(0)
    } else {
        next_duration - now
    };

    // Set timer_id if currently set timer is sooner
    let timer_id = schedule
        .timer_id
        .map(|timer_id| {
            if let Ok((timer_duration, _timer_arg)) = ctx.get_timer_info::<String>(timer_id) {
                if timer_duration > next_duration {
                    ctx.stop_timer::<String>(timer_id).ok();
                    None
                } else {
                    schedule.timer_id
                }
            } else {
                schedule.timer_id
            }
        })
        .flatten();

    if timer_id.is_some() {
        return;
    }

    let new_timer_id = ctx.create_timer(next_duration, exec_due_tasks, schedule_key);
    schedule.timer_id = Some(new_timer_id);
}

fn event_is_restore(event_type: redis_module::NotifyEvent, event: &str) -> bool {
    // TODO wait for loaded support:
    // (event_type == redis_module::NotifyEvent::GENERIC && event == "restore")
    //    || (event_type == redis_module::NotifyEvent::LOADED && event == "loaded")
    event_type == redis_module::NotifyEvent::GENERIC && event == "restore"
}

fn handle_rdb_loading(
    ctx: &Context,
    event_type: redis_module::NotifyEvent,
    event: &str,
    key: &str,
) {
    if !event_is_restore(event_type, event) {
        return;
    }

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let redis_key = ctx.open_key_writable(&key);

    if let Ok(Some(value)) = redis_key.get_value::<ScheduleDataType>(&SCHEDULE_DATA_TYPE) {
        update_timer(&ctx, key.to_string(), value, now);
    }
}

redis_module! {
    name: "ReDelay",
    version: 1,
    data_types: [
        SCHEDULE_DATA_TYPE,
    ],
    commands: [
        ["schedule.add", commands::add, "write", 1,1,1],
        ["schedule.exec", commands::exec, "write", 1,1,1],
        ["schedule.rem", commands::rem, "write", 1,1,1],
        ["schedule.replicate", commands::replicate, "write", 1,1,1],
        ["schedule.scan", commands::scan, "readonly", 1,1,1],
        ["schedule.incrby", commands::incrby, "write", 1,1,1],
        ["schedule.decrby", commands::decrby, "write", 1,1,1],
    ],
    event_handlers: [
        // TODO wait for @LOADED support
        // [@LOADED @GENERIC: handle_rdb_loading]
        [@GENERIC: handle_rdb_loading]
    ]
}
