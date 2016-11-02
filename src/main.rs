extern crate winapi;
#[cfg(windows)]
extern crate widestring;
extern crate serde;
extern crate serde_json;
extern crate hyper;
extern crate toml;

mod pdh_wrapper;

use std::env;
use std::path::Path;
use std::error::Error;
use pdh_wrapper::*;
use serde_json::builder;
use hyper::client;

fn main() {
    let config = env::current_dir()
        .map_err(|e| "ERROR CAN'T GET CURRENT DIR".to_string())
        .and_then(|c| open_config(c.join("config.toml").as_path()));

    let element_list =
        vec![PdhCounterPathElement::new(String::from("Memory"),
                                        String::from("Available Mbytes"),
                                        PdhCounterPathElementOptions { ..Default::default() })];

    let pdhc = PdhController::new(element_list).expect("Can't create Metrics Collector");
    let client = hyper::Client::new();
    for item in pdhc.into_iter().map(|v| v.to_json().to_string()) {
        println!("{}", item);
        client.post("https://hogehoge.com/perf")
            .body(item.as_str())
            .send()
            .unwrap();
    }
}

fn open_config(file_path: &Path) -> Result<toml::Table, String> {
    use std::fs::File;
    use std::io::prelude::*;

    let mut f = try!(File::open(file_path).map_err(|e| "Can't Open file".to_string()));
    let mut buffer = String::new();

    try!(f.read_to_string(&mut buffer).map_err(|e| "Can't read file".to_string()));
    toml::Parser::new(buffer.as_str()).parse().ok_or("Can't Open file".to_string())
}

impl PdhCollectValue {
    fn to_json(&self) -> serde_json::Value {
        builder::ObjectBuilder::new()
            .insert("object_name".to_string(), self.element.object_name.as_str())
            .insert("counter_name".to_string(),
                    self.element.counter_name.as_str())
            .insert("value".to_string(), &self.value)
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
