extern crate winapi;

mod pdh_wrapper;

fn main() {

    let element_list = vec![pdh_wrapper::PdhCounterPathElement::new(String::from("Memory"),
                                                                    String::from("Available \
                                                                                  Mbytes"))];

    let path_list = element_list.into_iter()
        .filter_map(|e| pdh_wrapper::pdh_make_counter_path(&e).ok())
        .collect::<Vec<_>>();

    let pdhc = pdh_wrapper::PdhController::new(&path_list).expect("Can't create Metrics Collector");
    println!("{:?}",
             pdhc.into_iter().map(|v| pdh_value2str(v)).collect::<Vec<_>>());
}

fn pdh_value2str(v: pdh_wrapper::PdhValue) -> String {

    match v {
        pdh_wrapper::PdhValue::LongLong(ll) => format!("{:.3}", ll),
        pdh_wrapper::PdhValue::Long(l) => format!("{:.3}", l),
        pdh_wrapper::PdhValue::Double(d) => format!("{:.3}", d),
        pdh_wrapper::PdhValue::Str(s) => s,
    }
}
