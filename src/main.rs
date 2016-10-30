extern crate winapi;
#[cfg(windows)]
extern crate widestring;
extern crate serde;
extern crate serde_json;

mod pdh_wrapper;

use pdh_wrapper::*;
use serde::ser;
use serde_json::Value;
use serde_json::builder;

fn main() {
    let element_list =
        vec![PdhCounterPathElement::new(String::from("Memory"),
                                        String::from("Available Mbytes"),
                                        PdhCounterPathElementOptions { ..Default::default() })];

    let pdhc = PdhController::new(element_list).expect("Can't create Metrics Collector");
    for item in pdhc.into_iter().map(|v| v.to_json()) {
        println!("{}", item);
    }
}

impl PdhCollectValue {
    fn to_json(&self) -> serde_json::Value {
        let mut obj = builder::ObjectBuilder::new();
        obj.insert("object_name".to_string(), self.element.object_name.as_str())
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

impl ser::Serialize for PdhValue {
    #[inline]
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: ser::Serializer
    {
        match *self {
            PdhValue::LongLong(ref ll) => serializer.serialize_i64(*ll),
            PdhValue::Long(ref l) => serializer.serialize_i64(*l as i64),
            PdhValue::Double(ref d) => serializer.serialize_f64(*d),
            PdhValue::Str(ref s) => serializer.serialize_str(s),
        }
    }
}