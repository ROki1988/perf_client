extern crate winapi;
extern crate pdh;

use winapi::pdh::*;

fn main() {
    let path_list = vec!["\\Memory\\Available Mbytes"];

    if let Some(pdhc) = PdhController::new(path_list) {

        let m = pdhc.current_values();

        println!("{:?}", m);
    }
}


#[derive(Debug)]
struct PdhController {
    hquery: winapi::PDH_HQUERY,
    hcounters: Vec<winapi::PDH_HCOUNTER>,
}

impl PdhController {
    fn new(path: Vec<&str>) -> Option<PdhController> {
        pdh_open_query()
            .map(|q| {
                let cs = path.into_iter()
                    .filter_map(|p| pdh_add_counter(q, p).ok())
                    .collect();
                PdhController {
                    hquery: q,
                    hcounters: cs,
                }
            })
            .ok()
    }

    fn current_values(&self) -> Vec<PdhValue> {
        pdh_collect_query_data(self.hquery);
        self.hcounters
            .iter()
            .filter_map(|&c| pdh_get_formatted_counter_value(c, PDH_FMT_DOUBLE).ok())
            .collect()
    }
}

impl IntoIterator for PdhController {
    type Item = PdhValue;
    type IntoIter = PdhControllerIntoIterator;

    fn into_iter(self) -> Self::IntoIter {
        PdhControllerIntoIterator {
            pdhc: self,
            index: 0,
        }
    }
}

impl Drop for PdhController {
    fn drop(&mut self) {
        pdh_close_query(self.hquery);
    }
}

struct PdhControllerIntoIterator {
    pdhc: PdhController,
    index: usize,
}

impl Iterator for PdhControllerIntoIterator {
    type Item = PdhValue;
    fn next(&mut self) -> Option<PdhValue> {
        if self.index == 0 {
            pdh_collect_query_data(self.pdhc.hquery);
        }

        let v = self.pdhc
            .hcounters
            .get(self.index)
            .map(|c| pdh_get_formatted_counter_value(*c, PDH_FMT_DOUBLE).ok());
        match v {
            Some(Some(a)) => Some(a),
            _ => None,
        }
    }
}


#[derive(Debug)]
enum PdhValue {
    LongLong(i64),
    Long(i32),
    Double(f64),
    Str(String),
}

fn pdh_open_query() -> Result<winapi::PDH_HQUERY, winapi::PDH_STATUS> {
    use std::ptr;
    let mut hquery = winapi::INVALID_HANDLE_VALUE;
    unsafe {
        let ret = pdh::PdhOpenQueryW(ptr::null(), 0, &mut hquery);
        if winapi::winerror::SUCCEEDED(ret) {
            Ok(hquery)
        } else {
            Err(ret)
        }
    }
}

fn pdh_collect_query_data(hquery: winapi::PDH_HQUERY) -> bool {
    unsafe { winapi::winerror::SUCCEEDED(pdh::PdhCollectQueryData(hquery)) }
}

fn pdh_get_formatted_counter_value(hcounter: winapi::PDH_HCOUNTER,
                                   format: winapi::DWORD)
                                   -> Result<PdhValue, winapi::PDH_STATUS> {
    let mut s = winapi::PDH_FMT_COUNTERVALUE {
        CStatus: 0,
        largeValue: 0,
    };
    unsafe {
        let mut devnul: winapi::DWORD = 0;
        let ret = pdh::PdhGetFormattedCounterValue(hcounter, format, &mut devnul, &mut s);
        if winapi::winerror::SUCCEEDED(ret) {
            Ok(to_value(s, format))
        } else {
            Err(ret)
        }
    }
}

fn to_value(s: PDH_FMT_COUNTERVALUE, format: winapi::DWORD) -> PdhValue {
    unsafe {
        match format {
            PDH_FMT_DOUBLE => PdhValue::Double(*s.doubleValue()),
            PDH_FMT_LONG => PdhValue::Long(*s.longValue()),
            PDH_FMT_LONGLONG => PdhValue::LongLong(*s.largeValue()),
        }
    }
}

fn pdh_close_query(hquery: PDH_HQUERY) -> Result<(), winapi::PDH_STATUS> {
    unsafe {
        let ret = pdh::PdhCloseQuery(hquery);
        if winapi::winerror::SUCCEEDED(ret) {
            Ok(())
        } else {
            Err(ret)
        }
    }
}

fn pdh_add_counter(hquery: winapi::PDH_HQUERY,
                   counter_path: &str)
                   -> Result<winapi::PDH_HCOUNTER, winapi::PDH_STATUS> {
    let mut hcounter = winapi::INVALID_HANDLE_VALUE;
    unsafe {
        let ret = pdh::PdhAddCounterW(hquery,
                                      to_wide_chars(counter_path).as_ptr(),
                                      0,
                                      &mut hcounter);
        if winapi::winerror::SUCCEEDED(ret) {
            Ok(hcounter)
        } else {
            Err(ret)
        }
    }
}

fn to_wide_chars(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(s).encode_wide().chain(Some(0).into_iter()).collect::<Vec<_>>()
}
