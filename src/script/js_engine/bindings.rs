use crate::common::{BoxErrResult, RequestData, ResponseData};
use bytes::Bytes;
use log::*;
use rusty_v8 as v8;
use std::collections::HashMap;
use std::ffi::c_void;

pub(crate) fn make_response<'s>(
    scope: &mut impl v8::ToLocal<'s>,
    context: v8::Local<v8::Context>,
    source: v8::Local<'s, v8::Value>,
) -> BoxErrResult<ResponseData> {
    if source.is_object() {
        let obj = source.to_object(scope).unwrap();
        let status_key = v8::String::new(scope, "status").unwrap();
        let status = obj.get(scope, context, status_key.into());
        let status = status
            .map(|s| s.int32_value(scope).unwrap_or(200))
            .unwrap_or(200) as u16;
        let header_key = v8::String::new(scope, "headers").unwrap();
        let headers = obj.get(scope, context, header_key.into());

        let mut header_map = HashMap::<String, String>::new();
        if let Some(headers) = headers {
            if headers.is_object() {
                let headers = headers.to_object(scope).unwrap();
                let header_keys = headers.get_own_property_names(scope, context);
                let header_keys_len = header_keys.length();
                if header_keys_len > 0 {
                    for i in 0..header_keys_len {
                        let key_obj = header_keys.get_index(scope, context, i).unwrap();
                        let key = key_obj.to_string(scope).unwrap();
                        let key_str = key.to_rust_string_lossy(scope).trim().to_string();
                        if !key_str.is_empty() {
                            if let Some(value) = headers.get(scope, context, key_obj) {
                                let value_str = value
                                    .to_string(scope)
                                    .unwrap()
                                    .to_rust_string_lossy(scope)
                                    .trim()
                                    .to_string();
                                if !value_str.is_empty() {
                                    debug!("[JS]  Set header {}={}", key_str, value_str);
                                    header_map.insert(key_str, value_str);
                                }
                            }
                        }
                    }
                }
            }
        }

        let body_key = v8::String::new(scope, "body").unwrap();
        let body = obj.get(scope, context, body_key.into());
        if body.is_none() {
            Ok(ResponseData {
                status: 404,
                headers: header_map,
                body: Bytes::from("Script returns empty content"),
            })
        } else {
            let body_str = body
                .unwrap()
                .to_string(scope)
                .unwrap()
                .to_rust_string_lossy(scope);
            Ok(ResponseData {
                status,
                headers: header_map,
                body: Bytes::from(body_str),
            })
        }
    } else if source.is_string() {
        let eval_str = source.to_string(scope).unwrap().to_rust_string_lossy(scope);
        Ok(ResponseData {
            status: 200,
            headers: HashMap::new(),
            body: Bytes::from(eval_str),
        })
    } else {
        Ok(ResponseData {
            status: 404,
            headers: HashMap::new(),
            body: Bytes::from("Script returns empty content"),
        })
    }
}

fn uri_accessor(
    scope: v8::PropertyCallbackScope,
    _name: v8::Local<v8::Name>,
    _args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let v8_isolate = scope.isolate();
    let request = unsafe { &*(v8_isolate.get_data(1) as *mut RequestData) };
    rv.set(v8::String::new(scope, &request.uri).unwrap().into());
}

fn query_accessor(
    scope: v8::PropertyCallbackScope,
    _name: v8::Local<v8::Name>,
    _args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let v8_isolate = scope.isolate();
    let request = unsafe { &*(v8_isolate.get_data(1) as *mut RequestData) };
    rv.set(v8::String::new(scope, &request.query).unwrap().into());
}

fn body_accessor(
    scope: v8::PropertyCallbackScope,
    _name: v8::Local<v8::Name>,
    _args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let v8_isolate = scope.isolate();
    let request = unsafe { &*(v8_isolate.get_data(1) as *mut RequestData) };
    rv.set(
        v8::String::new(scope, &String::from_utf8(request.body.to_vec()).unwrap())
            .unwrap()
            .into(),
    )
}

fn headers_accessor(
    scope: v8::FunctionCallbackScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let v8_isolate = scope.isolate();
    let request = unsafe { &*(v8_isolate.get_data(1) as *mut RequestData) };

    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let v8str = args
        .get(0)
        .to_string(scope)
        .unwrap_or(v8::String::empty(scope))
        .to_rust_string_lossy(scope);
    let arr: Vec<v8::Local<v8::Value>> = request
        .headers
        .get_all(v8str)
        .iter()
        .map(|value| {
            v8::String::new(scope, value.to_str().unwrap_or(""))
                .unwrap()
                .into()
        })
        .collect();
    let result = v8::Array::new_with_elements(scope, &arr);
    rv.set(result.into());
}

pub(crate) fn make_request<'s>(
    scope: &mut impl v8::ToLocal<'s>,
    context: v8::Local<v8::Context>,
    request: RequestData,
) -> BoxErrResult<()> {
    let boxed_request = Box::new(request);
    let raw_request = Box::into_raw(boxed_request);
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let v8_isolate = scope.isolate();
    unsafe { v8_isolate.set_data(1, raw_request as *mut c_void) }

    let obj_name = v8::String::new(scope, "request").unwrap();
    let headers_name = v8::String::new(scope, "header").unwrap();
    let body_name = v8::String::new(scope, "body").unwrap();
    let uri_name = v8::String::new(scope, "uri").unwrap();
    let query_name = v8::String::new(scope, "query").unwrap();
    let mut request_temp = v8::ObjectTemplate::new(scope);
    request_temp.set_accessor(uri_name, uri_accessor);
    request_temp.set_accessor(query_name, query_accessor);
    request_temp.set_accessor(body_name, body_accessor);
    let headers_func = v8::FunctionTemplate::new(scope, headers_accessor);
    request_temp.set_with_attr(
        headers_name.into(),
        headers_func.into(),
        v8::READ_ONLY + v8::DONT_ENUM + v8::DONT_DELETE,
    );
    let global = context.global(scope);
    let request_inst = request_temp.new_instance(scope, context).unwrap();
    global.set(context, obj_name.into(), request_inst.into());
    Ok(())
}

pub(crate) fn init_context<'s>(scope: &mut impl v8::ToLocal<'s>) -> v8::Local<'s, v8::Context> {
    let mut hs = v8::EscapableHandleScope::new(scope);
    let scope = hs.enter();

    let context = v8::Context::new(scope);
    let global = context.global(scope);

    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

    // add console log/error handler
    let console_log = v8::FunctionTemplate::new(scope, js_log);
    let console_error = v8::FunctionTemplate::new(scope, js_error);
    let console_key =
        v8::String::new_from_utf8(scope, "console".as_bytes(), v8::NewStringType::Normal).unwrap();
    let console_log_key =
        v8::String::new_from_utf8(scope, "log".as_bytes(), v8::NewStringType::Normal).unwrap();
    let console_error_key =
        v8::String::new_from_utf8(scope, "error".as_bytes(), v8::NewStringType::Normal).unwrap();
    let console_obj = v8::ObjectTemplate::new(scope);
    console_obj.set_with_attr(
        console_log_key.into(),
        console_log.into(),
        v8::READ_ONLY + v8::DONT_ENUM + v8::DONT_DELETE,
    );
    console_obj.set_with_attr(
        console_error_key.into(),
        console_error.into(),
        v8::READ_ONLY + v8::DONT_ENUM + v8::DONT_DELETE,
    );
    let console_instance = console_obj.new_instance(scope, context).unwrap();
    global.set(context, console_key.into(), console_instance.into());

    scope.escape(context)
}

pub(crate) fn js_log(
    scope: v8::FunctionCallbackScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let v8str = args
        .get(0)
        .to_string(scope)
        .unwrap_or(v8::String::empty(scope));
    let rstr = v8str.to_rust_string_lossy(scope);
    info!("[JS]  log: {}", rstr);
    rv.set(v8str.into())
}

pub(crate) fn js_error(
    scope: v8::FunctionCallbackScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let v8str = args
        .get(0)
        .to_string(scope)
        .unwrap_or(v8::String::empty(scope));
    let rstr = v8str.to_rust_string_lossy(scope);
    error!("[JS]  err: {}", rstr);
    rv.set(v8str.into())
}
