use redis_module::Context;

pub(crate) fn validate_command(ctx: &Context, args: &Vec<String>) {
    if !ctx.is_keys_position_request() {
        return;
    }

    let empty = "".to_string();
    let offset = 2;
    match args.first().unwrap_or(&empty).to_uppercase().as_str() {
        "LPUSH" | "RPUSH" => ctx.key_at_pos(offset + 1),
        "PING" | _ => {}
    }
}
