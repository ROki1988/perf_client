extern crate winapi;
#[cfg(windows)]
extern crate widestring;
extern crate serde;
extern crate serde_json;
extern crate toml;
extern crate rustc_serialize;

mod pdh_wrapper;

use std::env;
use std::path::Path;
use std::thread;
use std::sync::mpsc;

use pdh_wrapper::*;

use serde_json::builder;
use serde::ser;

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

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let pdhc = PdhController::new(element_list).expect("Can't Create Collector");
        loop {
            tx.send(pdhc.iter().collect::<Vec<_>>()).unwrap();
        }
    });

    loop {
        match rx.recv() {
            Ok(v) => {
                for item in v {
                    println!("{}", item.to_string());
                }
            }
            Err(_) => return,
        }
    }
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
