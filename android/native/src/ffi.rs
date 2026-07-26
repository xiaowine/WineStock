//! Android JNI 导出函数。
//!
//! 每个入口只交换 UTF-8 JSON，并捕获 Rust panic；无法创建 Java String 时返回 null，
//! Kotlin loader/manager 负责把该情况转换为 `native_library_unavailable`。

use std::panic::AssertUnwindSafe;

use jni::{
    errors::LogErrorAndDefault,
    objects::{JClass, JString},
    EnvUnowned,
};

use crate::{
    default_runtime_config_json, initialize_json, restart_local_service_json, runtime_state_json,
    shutdown_engine_json, start_local_service_json, stop_local_service_json,
    validate_runtime_config_json,
};

#[no_mangle]
pub extern "system" fn Java_winestock_xiaowine_cc_core_NativeCoreBridge_nativeInitialize<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> JString<'local> {
    init_android_logger();
    call_without_input(&mut env, initialize_json)
}

#[no_mangle]
pub extern "system" fn Java_winestock_xiaowine_cc_core_NativeCoreBridge_nativeDefaultRuntimeConfig<
    'local,
>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> JString<'local> {
    call_without_input(&mut env, default_runtime_config_json)
}

#[no_mangle]
pub extern "system" fn Java_winestock_xiaowine_cc_core_NativeCoreBridge_nativeValidateRuntimeConfig<
    'local,
>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    input: JString<'local>,
) -> JString<'local> {
    call_with_input(&mut env, input, validate_runtime_config_json)
}

#[no_mangle]
pub extern "system" fn Java_winestock_xiaowine_cc_core_NativeCoreBridge_nativeStartLocalService<
    'local,
>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    input: JString<'local>,
) -> JString<'local> {
    call_with_input(&mut env, input, start_local_service_json)
}

#[no_mangle]
pub extern "system" fn Java_winestock_xiaowine_cc_core_NativeCoreBridge_nativeStopLocalService<
    'local,
>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> JString<'local> {
    call_without_input(&mut env, stop_local_service_json)
}

#[no_mangle]
pub extern "system" fn Java_winestock_xiaowine_cc_core_NativeCoreBridge_nativeRestartLocalService<
    'local,
>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    input: JString<'local>,
) -> JString<'local> {
    call_with_input(&mut env, input, restart_local_service_json)
}

#[no_mangle]
pub extern "system" fn Java_winestock_xiaowine_cc_core_NativeCoreBridge_nativeGetRuntimeState<
    'local,
>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> JString<'local> {
    call_without_input(&mut env, runtime_state_json)
}

#[no_mangle]
pub extern "system" fn Java_winestock_xiaowine_cc_core_NativeCoreBridge_nativeShutdownEngine<
    'local,
>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> JString<'local> {
    call_without_input(&mut env, shutdown_engine_json)
}

fn call_without_input<'local>(
    env: &mut EnvUnowned<'local>,
    operation: fn() -> String,
) -> JString<'local> {
    let output =
        std::panic::catch_unwind(AssertUnwindSafe(operation)).unwrap_or_else(|_| panic_response());
    java_string(env, output)
}

fn call_with_input<'local>(
    env: &mut EnvUnowned<'local>,
    input: JString<'local>,
    operation: fn(&str) -> String,
) -> JString<'local> {
    env.with_env(|env| -> jni::errors::Result<JString<'local>> {
        let output = match input.try_to_string(env) {
            Ok(input) => std::panic::catch_unwind(AssertUnwindSafe(|| operation(&input)))
                .unwrap_or_else(|_| panic_response()),
            Err(_) => crate::contract::encode_response::<()>(Err(
                crate::error::NativeError::invalid_payload(),
            )),
        };
        JString::from_str(env, output)
    })
    .resolve::<LogErrorAndDefault>()
}

fn java_string<'local>(env: &mut EnvUnowned<'local>, value: String) -> JString<'local> {
    env.with_env(|env| JString::from_str(env, value))
        .resolve::<LogErrorAndDefault>()
}

fn panic_response() -> String {
    crate::contract::encode_response::<()>(Err(crate::error::NativeError::new(
        "service_start_failed",
        "Android native 内部执行失败",
    )))
}

fn init_android_logger() {
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("WineStockCore")
            .with_max_level(log::LevelFilter::Info),
    );
    // Rust panic 默认只写 stderr，在 Android 上会无声丢失；转发到 logcat 便于定位请求线程 panic。
    static PANIC_HOOK: std::sync::Once = std::sync::Once::new();
    PANIC_HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            log::error!("core panic: {info}");
            previous(info);
        }));
    });
}
