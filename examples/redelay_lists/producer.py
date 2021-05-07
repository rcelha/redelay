"""
Clears timetable:result and schedule 10 tasks to append items to it
"""
import datetime
import redis
from redelay import commands

r = redis.Redis(port=6666)

commands.schedule_add(r, "timetable", datetime.timedelta(seconds=0), ["del", "timetable:result"])

for i in range(10):
    now = datetime.datetime.now()
    delta = datetime.timedelta(seconds=i * 2)
    commands.schedule_add(r, "timetable", delta, ["lpush", "timetable:result", now.timestamp()])
