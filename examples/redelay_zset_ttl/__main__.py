"""
Expire users from a chat room after 60 seconds
"""
import datetime
from time import sleep
import redis
from redelay import commands

r = redis.Redis(port=6666)
r.delete("chatroom:presence:{1}", "chatroom:cleaner:{1}")

for i in range(10):
    now = datetime.datetime.now()
    delay = datetime.timedelta(seconds=60)
    user_id = f"user:{i}"
    r.zadd("chatroom:presence:{1}", {user_id: datetime.datetime.now().timestamp()})
    commands.schedule_add(r, "chatroom:cleaner:{1}", delay, ["ZREM", "chatroom:presence:{1}", user_id])


while online_users := r.zcount("chatroom:presence:{1}", "-inf", "+inf"):
    print(f"{online_users} online")
    sleep(1)

print(f"{online_users} online")
