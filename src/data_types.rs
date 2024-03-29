use crate::skiplist_ext::{de_skiplist, ser_skiplist};

use redis_module::native_types::RedisType;
use redis_module::raw;
use serde::{Deserialize, Serialize};
use skiplist::ordered_skiplist::Iter;
use skiplist::OrderedSkipList;
use std::collections::HashMap;
use std::os::raw::{c_int, c_void};
use std::vec::Vec;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub timestamp: u64,
    pub args: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleDataType {
    // (Timestamp, TaskID)
    #[serde(serialize_with = "ser_skiplist", deserialize_with = "de_skiplist")]
    timetable: OrderedSkipList<(u64, String)>,
    // TaskID : ARGV[...]
    tasks: HashMap<String, Task>,

    #[serde(skip)]
    pub timer_id: Option<u64>,
}

impl ScheduleDataType {
    pub fn new() -> Self {
        let timetable = OrderedSkipList::new();
        ScheduleDataType {
            tasks: HashMap::new(),
            timetable,
            timer_id: None,
        }
    }

    pub fn add_task(&mut self, timestamp: u64, task_id: String, args: Vec<String>) {
        self.timetable.insert((timestamp, task_id.clone()));
        self.tasks.insert(task_id, Task { timestamp, args });
    }

    pub fn del_task(&mut self, task_id: String) -> Option<Task> {
        let task = self.tasks.remove(&task_id)?;
        self.timetable.remove(&(task.timestamp, task_id));
        Some(task)
    }

    pub fn get_min_timestamp(&self) -> Option<u64> {
        self.timetable
            .front()
            .map(|(timestamp, _task_id)| *timestamp)
    }

    #[cfg(test)]
    fn pop_by_timestamp(&mut self, limit: u64) -> Option<(u64, String, Vec<String>)> {
        let (head_timestamp, _) = self.timetable.front()?;

        if *head_timestamp > limit {
            return None;
        }
        let (head_timestamp, head_task_id) = self.timetable.pop_front()?;
        let task = self.tasks.remove(&head_task_id)?;
        Some((head_timestamp, head_task_id, task.args))
    }

    pub fn timetable_iter(&self) -> Iter<(u64, String)> {
        self.timetable.iter()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn change_timestamp_by(
        &mut self,
        task_id: String,
        callback: impl Fn(u64) -> u64,
    ) -> Option<u64> {
        let mut task = self.tasks.get_mut(&task_id)?;
        self.timetable
            .remove_first(&(task.timestamp, task_id.clone()));
        task.timestamp = callback(task.timestamp);
        self.timetable.insert((task.timestamp, task_id));
        Some(task.timestamp)
    }

    pub fn incr(&mut self, task_id: String, value: u64) -> Option<u64> {
        self.change_timestamp_by(task_id, |x| x + value)
    }

    pub fn decr(&mut self, task_id: String, value: u64) -> Option<u64> {
        self.change_timestamp_by(task_id, |x| x - value)
    }

    pub fn to_vec(&self) -> Vec<(u64, String, Vec<String>)> {
        let mut ret = Vec::with_capacity(self.timetable.len());
        for (timestamp, task_id) in &self.timetable {
            let final_task_id = task_id.clone();
            let args = if let Some(task) = self.tasks.get(&final_task_id) {
                task.args.clone()
            } else {
                Vec::new()
            };
            ret.push((*timestamp, final_task_id, args));
        }
        ret
    }
}

unsafe extern "C" fn free(value: *mut c_void) {
    Box::from_raw(value as *mut ScheduleDataType);
}

#[allow(non_snake_case, unused)]
pub extern "C" fn rdb_load(rdb: *mut raw::RedisModuleIO, encver: c_int) -> *mut c_void {
    let data = raw::load_string(rdb);
    let schedule: ScheduleDataType = serde_json::from_str(&data).unwrap();
    Box::into_raw(Box::new(schedule)) as *mut c_void
}

pub extern "C" fn rdb_save(rdb: *mut raw::RedisModuleIO, value: *mut c_void) {
    let schedule = unsafe { &*(value as *mut ScheduleDataType) };
    raw::save_string(rdb, &serde_json::to_string(&schedule).unwrap());
}

pub static SCHEDULE_DATA_TYPE: RedisType = RedisType::new(
    "schedulet",
    0,
    raw::RedisModuleTypeMethods {
        version: raw::REDISMODULE_TYPE_METHOD_VERSION as u64,

        rdb_save: Some(rdb_save),
        rdb_load: Some(rdb_load),

        aof_rewrite: None,
        free: Some(free),

        mem_usage: None,
        digest: None,

        aux_load: None,
        aux_save: None,
        aux_save_triggers: 0,

        free_effort: None,
        unlink: None,
        copy: None,
        defrag: None,
    },
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_min_timestamp() {
        let mut schedule = ScheduleDataType::new();

        schedule.add_task(10, "task-a".to_string(), vec![]);
        assert_eq!(schedule.get_min_timestamp(), Some(10));

        schedule.add_task(1, "task-b".to_string(), vec![]);
        assert_eq!(schedule.get_min_timestamp(), Some(1));

        schedule.pop_by_timestamp(5);
        assert_eq!(schedule.get_min_timestamp(), Some(10));
    }

    #[test]
    fn odering() {
        let mut schedule = ScheduleDataType::new();
        schedule.add_task(10, "task-a".to_string(), vec![]);
        schedule.add_task(1, "task-b".to_string(), vec![]);
        schedule.add_task(100, "task-c".to_string(), vec![]);

        let name = |x: (u64, String, Vec<String>)| x.1;

        assert_eq!(
            schedule.pop_by_timestamp(200).map(name),
            Some("task-b".into())
        );
        assert_eq!(
            schedule.pop_by_timestamp(200).map(name),
            Some("task-a".into())
        );
        assert_eq!(
            schedule.pop_by_timestamp(200).map(name),
            Some("task-c".into())
        );

        assert_eq!(schedule.pop_by_timestamp(200), None);
    }

    #[test]
    fn del() {
        let mut schedule = ScheduleDataType::new();
        schedule.add_task(10, "task-a".to_string(), vec!["A".to_string()]);
        schedule.add_task(1, "task-b".to_string(), vec!["B".to_string()]);
        schedule.add_task(100, "task-c".to_string(), vec!["C".to_string()]);
        assert_eq!(schedule.len(), 3);

        let task_b = schedule.del_task("task-b".to_string());
        assert_eq!(task_b.unwrap().args, vec!["B".to_string()]);
        assert_eq!(schedule.len(), 2);
    }

    #[test]
    fn incr() {
        let mut schedule = ScheduleDataType::new();

        schedule.add_task(10, "task-a".to_string(), vec!["A".to_string()]);
        schedule.add_task(1, "task-b".to_string(), vec!["B".to_string()]);
        assert_eq!(schedule.get_min_timestamp(), Some(1));

        assert_eq!(schedule.incr("task-b".to_string(), 5), Some(6));
        assert_eq!(schedule.get_min_timestamp(), Some(6));
        assert_eq!(schedule.len(), 2);
    }

    #[test]
    fn decr() {
        let mut schedule = ScheduleDataType::new();

        schedule.add_task(10, "task-a".to_string(), vec!["A".to_string()]);
        schedule.add_task(6, "task-b".to_string(), vec!["B".to_string()]);

        assert_eq!(schedule.get_min_timestamp(), Some(6));

        assert_eq!(schedule.decr("task-b".to_string(), 5), Some(1));
        assert_eq!(schedule.get_min_timestamp(), Some(1));
        assert_eq!(schedule.len(), 2);
    }

    #[test]
    fn serde() {
        let mut schedule = ScheduleDataType::new();
        schedule.add_task(10, "task-a".to_string(), vec!["A".to_string()]);
        schedule.add_task(6, "task-b".to_string(), vec!["B".to_string()]);

        let ser_schedule = serde_json::to_string(&schedule).unwrap();
        let de_schedule: ScheduleDataType = serde_json::from_str(&ser_schedule).unwrap();
        assert_eq!(schedule.tasks, de_schedule.tasks);
        assert_eq!(schedule.timetable, de_schedule.timetable);
    }
}
