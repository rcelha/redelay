# ReDelay

Schedule commands in Redis.

⚠️ _This is a work in progress_ ⚠️

## Example

See the `./examples` folder.

## Commands

### SCHEDULE.ADD KEY DELAY COMMAND [ARG ...]

Schedule a command (`COMMAND` + `ARGS`) to execute in `DELAY` seconds.
This returns the task's id (a v4 uuid).

### SCHEDULE.REM KEY TASK-ID

Remove a task from a schedule by its id.

### SCHEDULE.SCAN KEY

List all the tasks (id, timestamp, command) present in a schedule (do not includes the executed ones).

### SCHEDULE.INCRBY KEY TASK-ID SECONDS

Delay a task even further

### SCHEDULE.DECRBY KEY TASK-ID SECONDS

Speed up a task

### SCHEDULE.REPLICATE KEY TIMESTAMP TASK-ID COMMAND [ARG ...]

Internal command to replicate/restore schedule from/to AOF.

### SCHEDULE.EXEC (CAUSES THE TASK SIDE EFFECT)

Executes a task, triggering its command.

## Build and run

You can build the library with cargo:

```sh
cargo build --release
```

Simply load it in Redis as any other module:

```sh
redis-server --loadmodule ./target/release/libredelay.so
```

## Running tests

```sh
cargo t --features test
```

... Or ...

```sh
make test
```

## Running all integration tests (in docker)

```sh
make start-all
```

## TODO

- [x] RDB Support
- [x] Validate commands on receive
- [x] Cluster support
- [ ] Do not start/execute timers on replicas?
- [ ] Test coverage
- [x] Fix all clippy warnings
- [ ] Suppress clippy error from redis-module
- [ ] Example with streams
- [ ] Create timers after AOF restore
- [ ] Move timer create/update into event module
- [ ] RDB integration tests
- [ ] Test for key removed
- [ ] Test for cluster replication
- [ ] Dead-letter
