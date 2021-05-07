import redis
import datetime
import typing


def schedule_add(
    r: redis.Redis,
    key_: str,
    delay: datetime.timedelta,
    args: typing.List[str],
) -> None:
    res = r.execute_command(
        "schedule.add", key_, int(delay.total_seconds()), *args
    )
    return res


def schedule_scan(
    r: redis.Redis,
    key_: str,
) -> None:
    res = r.execute_command(
        "schedule.scan", key_
    )
    return res

def schedule_rem(
    r: redis.Redis,
    key_: str,
) -> None:
    res = r.execute_command(
        "schedule.rem", key_
    )
    return res
