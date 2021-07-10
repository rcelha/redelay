use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    ptr::null,
};

use redis_module::{raw, Context, RedisError, RedisString};
use std::string::ToString;

pub trait ContextExt {
    fn replicate(&self, command: &str, args: &[&str]);
    fn get_command_keys(&self, args: &[String]) -> Result<Vec<i32>, RedisError>;
    fn get_server_info(&self, fields: &[String]) -> Result<HashMap<String, String>, RedisError>;
}

impl ContextExt for Context {
    fn replicate(&self, command: &str, args: &[&str]) {
        raw::replicate(self.ctx, command, args);
    }

    fn get_command_keys(&self, args: &[String]) -> Result<Vec<i32>, RedisError> {
        if args.len() < 1 {
            return Err(RedisError::WrongArity);
        };

        let redis_string_args: Vec<RedisString> = args
            .iter()
            .map(|s| RedisString::create(self.ctx, &s))
            .collect();

        let mut inner_args: Vec<*mut raw::RedisModuleString> =
            redis_string_args.iter().map(|s| s.inner).collect();

        let mut num_keys = -1;
        let ptr_keys = unsafe {
            raw::RedisModule_GetCommandKeys.unwrap()(
                self.ctx,
                inner_args.as_mut_ptr(),
                inner_args.len() as i32,
                &mut num_keys,
            )
        };
        if num_keys < 0 {
            return Err(RedisError::WrongArity);
        };
        let keys = unsafe { Vec::from_raw_parts(ptr_keys, num_keys as usize, num_keys as usize) };
        Ok(keys)
    }

    fn get_server_info(&self, fields: &[String]) -> Result<HashMap<String, String>, RedisError> {
        let mut ret = HashMap::with_capacity(fields.len());
        let server_info = unsafe { raw::RedisModule_GetServerInfo.unwrap()(self.ctx, null()) };
        for field in fields {
            let field_name = CString::new(field.as_str()).unwrap();
            let field_value = unsafe {
                let field =
                    raw::RedisModule_ServerInfoGetFieldC.unwrap()(server_info, field_name.as_ptr());
                CStr::from_ptr(field)
            };
            // TODO remove clone?
            ret.insert(field.clone(), field_value.to_str().unwrap().to_string());
        }
        Ok(ret)
    }
}
