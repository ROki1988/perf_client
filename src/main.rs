extern crate winapi;
#[cfg(windows)]
extern crate widestring;
extern crate serde;
extern crate serde_json;

mod pdh_wrapper;

use pdh_wrapper::*;
use serde_json::Map;

fn main() {

    let element_list =
        vec![PdhCounterPathElement::new(String::from("Memory"),
                                        String::from("Available Mbytes"),
                                        PdhCounterPathElementOptions { ..Default::default() })];

    let pdhc = PdhController::new(element_list).expect("Can't create Metrics Collector");
    for item in pdhc.into_iter().map(|v| v.to_string()) {
        println!("{}", item);
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
