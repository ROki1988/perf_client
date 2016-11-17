extern crate winapi;
#[cfg(windows)]
extern crate widestring;
extern crate serde;
extern crate serde_json;
extern crate toml;
extern crate rustc_serialize;
extern crate robots;

mod pdh_wrapper;

use std::env;
use std::path::Path;
use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use pdh_wrapper::*;

use serde_json::builder;
use serde::ser;

use robots::actors::{Actor, ActorSystem, ActorCell, ActorContext, Props, Message};

fn main() {
    let config = env::current_dir()
        .map_err(|e| format!("ERROR CAN'T GET CURRENT DIR: {}", e))
        .and_then(|c| open_config(c.join("config.toml").as_path()))
        .expect("Open Config File");

    let endpoint = config.get("Host")
        .into_iter()
        .flat_map(|ref os| os.as_str())
        .last()
        .expect("Find Host from Config");

    let element_list = config.get("element")
        .into_iter()
        .flat_map(|t_e| toml::decode::<Vec<PdhCounterPathElement>>(t_e.clone()))
        .last()
        .expect("Find Element from Config");

    let actor_system = ActorSystem::new("test".to_owned());

    let pdhc = |es: Vec<PdhCounterPathElement>| -> MetricsCollector {
        MetricsCollector::new(es).expect("Can't create Metrics Collector")
    };
    let props = Props::new(Arc::new(pdhc), element_list);
    let _actor = actor_system.actor_of(props, "metrics_collector".to_owned());

    actor_system.spawn_threads(1);
}

fn open_config(file_path: &Path) -> Result<toml::Table, String> {
    use std::fs::File;
    use std::io::prelude::*;

    let mut f = File::open(file_path).map_err(|e| format!("Can't Open file: {}", e))?;
    let mut buffer = String::new();

    f.read_to_string(&mut buffer).map_err(|e| format!("Can't read file: {}", e))?;
    toml::Parser::new(buffer.as_str()).parse().ok_or(format!("Can't parse file: {:?}", file_path))
}


impl PdhCollectValue {
    fn to_json(&self) -> serde_json::Value {
        fn build_with_option<T: ser::Serialize>(x: builder::ObjectBuilder,
                                                key: &str,
                                                value: &Option<T>)
                                                -> builder::ObjectBuilder {
            if let &Some(ref s) = value {
                x.insert(key, s)
            } else {
                x
            }
        };

        let result = builder::ObjectBuilder::new()
            .insert("object_name", &self.element.object_name)
            .insert("counter_name", &self.element.counter_name)
            .insert("value", &self.value);
        let result =
            build_with_option(result, "instance_name", &self.element.options.instance_name);
        let result = build_with_option(result, "machine_name", &self.element.options.machine_name);
        let result = build_with_option(result,
                                       "parent_instance",
                                       &self.element.options.parent_instance);
        build_with_option(result,
                          "instance_index",
                          &self.element.options.instance_index)
            .build()
    }
}

unsafe impl Sync for pdh_wrapper::PdhController {}

unsafe impl Send for pdh_wrapper::PdhController {}

#[derive(Debug)]
struct MetricsCollector {
    pdhc: PdhController,
}

impl MetricsCollector {
    fn new(element_list: Vec<PdhCounterPathElement>) -> Option<MetricsCollector> {
        PdhController::new(element_list).map(|c| MetricsCollector { pdhc: c })
    }
}

impl Actor for MetricsCollector {
    fn pre_start(&self, context: ActorCell) {
        let props = Props::new(Arc::new(Printer::new), ());
        let printer = context.actor_of(props, "printer".to_owned()).unwrap();
        context.tell(printer, self.pdhc.iter().next().unwrap().clone());
    }

    fn receive(&self, _message: Box<Any>, _context: ActorCell) {}
}

#[derive(Debug)]
struct Printer {}

impl Printer {
    fn new(_dummy: ()) -> Printer {
        Printer {}
    }
}

impl Actor for Printer {
    fn receive(&self, message: Box<Any>, context: ActorCell) {
        if let Ok(message) = Box::<Any>::downcast::<PdhCollectValue>(message) {
            println!("{}", message.to_string());
            context.stop(context.sender());
        }
    }
}

impl ToString for PdhCounterPathElement {
    fn to_string(&self) -> String {
        format!("{}\\{}", self.object_name, self.counter_name)
    }
}

impl ToString for PdhValue {
    fn to_string(&self) -> String {
        match *self {
            PdhValue::LongLong(ref ll) => format!("{:.3}", ll),
            PdhValue::Long(ref l) => format!("{:.3}", l),
            PdhValue::Double(ref d) => format!("{:.3}", d),
            PdhValue::Str(ref s) => s.clone(),
        }
    }
}
