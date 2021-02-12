"""
consumer.py

Remove 10 items from "timetable:result" and exit
"""
import datetime
import redis


r = redis.Redis(port=6666)

for i in range(10):
    now = datetime.datetime.now()
    _, timetable_item = r.blpop("timetable:result")
    item_time = datetime.datetime.fromtimestamp(float(timetable_item))
    print(f"{now} - timetable_item({item_time})")
