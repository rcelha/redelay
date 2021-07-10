#!/bin/bash

set -x

make builder
make stop-redis-cluster clear
make redis-cluster-d

REDIS_CLI="docker run -it --rm --network redelay_cluster_default redis redis-cli -c "

$REDIS_CLI -h redis-node-1 lrange bla{1} 0 -1
$REDIS_CLI -h redis-node-1 schedule.add foo{1} 10 lpush bla{1} oi
sleep 10
$REDIS_CLI -h redis-node-1 lrange bla{1} 0 -1

$REDIS_CLI -h redis-node-1 schedule.add foo{1} 30 lpush bla{1} tchau
make stop-redis-cluster
make redis-cluster-d
echo run make redis-cluster-logs
sleep 35
$REDIS_CLI -h redis-node-1 lrange bla{1} 0 -1
$REDIS_CLI -h redis-node-5 lrange bla{1} 0 -1
