use futures::executor::block_on;
use hirofa_utils::js_utils::adapters::proxies::JsProxy;
use hirofa_utils::js_utils::adapters::JsRealmAdapter;
use hirofa_utils::js_utils::facades::values::{JsValueConvertable, JsValueFacade};
use hirofa_utils::js_utils::facades::JsRuntimeFacade;
use hirofa_utils::js_utils::Script;
use log::LevelFilter;
use quickjs_runtime::builder::QuickJsRuntimeBuilder;
use quickjs_runtime::facades::QuickJsRuntimeFacade;
use quickjs_runtime::quickjsrealmadapter::QuickJsRealmAdapter;
use std::time::Duration;

fn main() {
    simple_logging::log_to_stderr(LevelFilter::Info);
    let rt = QuickJsRuntimeBuilder::new().build();
    block_on(run_examples(&rt));
}

async fn take_long() -> i32 {
    std::thread::sleep(Duration::from_millis(500));
    537
}

pub async fn run_examples(rt: &QuickJsRuntimeFacade) {
    let cb = JsValueFacade::new_callback(|args| {
        let a = args[0].get_i32();
        let b = args[1].get_i32();
        log::info!("rust cb was called with a:{} and b:{}", a, b);
        Ok(JsValueFacade::Null)
    });

    if let Err(err) = rt
        .js_function_invoke(
            None,
            &[],
            "setTimeout",
            vec![
                cb,
                10.to_js_value_facade(),
                12.to_js_value_facade(),
                13.to_js_value_facade(),
            ],
        )
        .await
    {
        println!("error_msg: {}", err.to_string());
    }

    std::thread::sleep(Duration::from_millis(20));
    log::info!("rust cb should have been called by now");

    rt.js_loop_realm_sync(None, |_rt_adapter, realm_adapter| {
        let proxy = JsProxy::new(&["com", "mystuff"], "MyProxy").add_static_method(
            "doSomething",
            |_rt_adapter, realm_adapter: &QuickJsRealmAdapter, _args| {
                realm_adapter.js_promise_create_resolving_async(
                    async { Ok(take_long().await) },
                    |realm_adapter, producer_result| realm_adapter.js_i32_create(producer_result),
                )
            },
        );
        realm_adapter
            .js_proxy_install(proxy, true)
            .ok()
            .expect("could not install proxy");
    });

    if let Err(err) = rt
        .js_eval(
            None,
            Script::new(
                "testMyProxy.js",
                "async function a() {\
                        console.log('a called at %s ms', new Date().getTime());\
                        let res = await com.mystuff.MyProxy.doSomething();\
                        console.log('a got result %s at %s ms', res, new Date().getTime());\
                       }; a();",
            ),
        )
        .await
    {
        println!("error_msg: {}", err.to_string());
    }

    std::thread::sleep(Duration::from_millis(600));
    log::info!("a should have been called by now");
}
