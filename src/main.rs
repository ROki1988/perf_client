extern crate winapi;
#[cfg(windows)]
extern crate widestring;
extern crate serde;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
extern crate toml;

mod pdh_wrapper;

use std::env;
use std::path::Path;
use std::thread;
use std::sync::mpsc;
use std::time::Duration;

use pdh_wrapper::*;

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
        .flat_map(|t_e| t_e.clone().try_into::<Vec<PdhCounterPathElement>>())
        .last()
        .expect("Find Element from Config");

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let pdhc = PdhController::new(element_list).expect("Can't Create Collector");
        loop {
            tx.send(pdhc.iter().collect::<Vec<_>>()).unwrap();
            thread::sleep(Duration::from_millis(1000))
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

fn open_config(file_path: &Path) -> Result<toml::value::Table, String> {
    use std::fs::File;
    use std::io::prelude::*;

    let mut f = File::open(file_path).map_err(|e| format!("Can't Open file: {}", e))?;
    let mut buffer = String::new();

    f.read_to_string(&mut buffer).map_err(|e| format!("Can't read file: {}", e))?;
    toml::from_str::<toml::value::Table>(buffer.as_str()).map_err(|_| format!("Can't parse file: {:?}", file_path))
}


impl PdhCollectValue {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "object_name": &self.element.object_name,
            "counter_name": &self.element.counter_name,
            "value": &self.value,
            "instance_name": &self.element.options.instance_name,
            "machine_name": &self.element.options.machine_name,
            "parent_instance": &self.element.options.parent_instance,
            "instance_index":  &self.element.options.instance_index
        })
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
