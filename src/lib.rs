#![allow(clippy::not_unsafe_ptr_arg_deref)] // TODO: remove this once redis_module stops causing the error

#[macro_use]
extern crate redis_module;
use redis_module::{Context, RedisResult};
use std::string::String;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::vec::Vec;

mod context_ext;
use context_ext::ContextExt;
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

// TODO is this the best way to get whether the node is a replica?
fn is_replica_node(ctx: &Context) -> bool {
    let server_info = ctx
        .get_server_info(&["cluster_enabled".to_string(), "role".to_string()])
        .unwrap();

    let cluster_enabled = server_info
        .get("cluster_enabled")
        .map(|x| x.as_str())
        .unwrap_or_else(|| "0")
        == "1";

    let role = server_info
        .get("role")
        .map(|x| x.as_str())
        .unwrap_or_else(|| "master");

    cluster_enabled && role != "master"
}

// Execute the due tasks and schedule the next execution
fn exec_due_tasks(ctx: &Context, schedule_key: String) {
    // Only execute the task on master nodes
    // It can return without scheduling the next timer
    // because it will schedule when it executes the
    // SCHEDULE.REM command
    if is_replica_node(ctx) {
        let msg = format!(
            "Skip execution for schedule '{}'. This is a replica node",
            schedule_key
        );
        ctx.log_notice(&msg);
        return;
    }

    let msg = format!("Executing due tasks for schedule '{}'", schedule_key);
    ctx.log_notice(&msg);

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let now_timestamp = now.as_secs();
    commands::exec_due(
        ctx,
        vec![
            "SCHEDULE.EXECDUE".to_string(),
            schedule_key,
            now_timestamp.to_string(),
        ],
    )
    .unwrap();
}

/// Updates a schedules's timer
///
/// If there is no next item, ignore the operation
/// If the current timer is later then the head, stop current timer
fn update_timer(
    ctx: &Context,
    schedule_key: String,
    schedule: &mut ScheduleDataType,
    now: Duration,
) {
    let next_timestamp = match schedule.get_min_timestamp() {
        Some(v) => v,
        _ => return, // No item in the schedule
    };

    let next_duration = Duration::from_secs(next_timestamp);
    let next_duration = if next_duration < now {
        // Set it to execute ASAP if the task's time is in the past
        Duration::from_secs(0)
    } else {
        next_duration - now
    };

    let create_a_new_timer = if let Some(timer_id) = schedule.timer_id {
        if let Ok((timer_duration, _timer_arg)) = ctx.get_timer_info::<String>(timer_id) {
            if timer_duration.as_nanos() == 0 {
                ctx.log_warning(format!("Found a timer ({}) for schedule '{}'. Its duration is zero, removing a scheduling another one", timer_id, schedule_key).as_str());
                // ctx.stop_timer::<String>(timer_id).ok();
                true
            } else if timer_duration > next_duration {
                ctx.stop_timer::<String>(timer_id).ok();
                true
            } else {
                // There is a valid timer. Do nothing
                false
            }
        } else {
            // Tho there is a timer id, I couldn't find it. Create a new one
            true
        }
    } else {
        // Task has no timer. Create one
        true
    };

    if !create_a_new_timer {
        return;
    }

    let new_timer_id = ctx.create_timer(next_duration, exec_due_tasks, schedule_key);
    schedule.timer_id = Some(new_timer_id);
}

///
/// Same as update_timer, but it gets the value from redis
///
/// If now is None, it will get the current system time
fn open_key_and_update_timer(ctx: &Context, schedule_key: String, now: Option<Duration>) {
    let now = now.unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).unwrap());
    let redis_key = ctx.open_key_writable(&schedule_key);
    let redis_value = redis_key.get_value::<ScheduleDataType>(&SCHEDULE_DATA_TYPE);

    if let Ok(Some(value)) = redis_value {
        update_timer(&ctx, schedule_key, value, now);
    }
}

fn event_is_restore(event_type: redis_module::NotifyEvent, event: &str) -> bool {
    (event_type == redis_module::NotifyEvent::GENERIC && event == "restore")
        || (event_type == redis_module::NotifyEvent::LOADED && event == "loaded")
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

    open_key_and_update_timer(ctx, key.to_string(), None);
}

redis_module! {
    name: "ReDelay",
    version: 1,
    data_types: [
        SCHEDULE_DATA_TYPE,
    ],
    commands: [
        ["schedule.add", commands::add, "write getkeys-api", 1,1,1],
        ["schedule.exec", commands::exec, "write", 1,1,1],
        ["schedule.execdue", commands::exec_due, "write", 1,1,1],
        ["schedule.rem", commands::rem, "write", 1,1,1],
        ["schedule.replicate", commands::replicate, "write getkeys-api", 1,1,1],
        ["schedule.scan", commands::scan, "readonly", 1,1,1],
        ["schedule.incrby", commands::incrby, "write", 1,1,1],
        ["schedule.decrby", commands::decrby, "write", 1,1,1],
    ],
    event_handlers: [
        [@LOADED @GENERIC: handle_rdb_loading]
    ]
}
