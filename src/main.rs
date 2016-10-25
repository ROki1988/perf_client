extern crate winapi;

mod pdh_wrapper;

use pdh_wrapper::*;

fn main() {

    let element_list = vec![PdhCounterPathElement::new(String::from("Memory"),
                                                       String::from("Available Mbytes"))];

    let path_list = element_list.into_iter()
        .filter_map(|e| pdh_make_counter_path(&e).ok())
        .collect::<Vec<_>>();

    let pdhc = PdhController::new(&path_list).expect("Can't create Metrics Collector");
    println!("{:?}",
             pdhc.into_iter().map(|v| v.to_string()).collect::<Vec<_>>());
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
