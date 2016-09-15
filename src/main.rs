extern crate winapi;
extern crate pdh;

use winapi::pdh::*;

fn main() {
    let path_list = vec!["\\Memory\\Available Mbytes"];
    if let Some(pdhc) = PdhControler::new(path_list) {
        
        let m = pdhc.get_current_values();

        println!("{:?}", m);
    }
}


#[derive(Debug)]
struct PdhControler {
    hquery: winapi::PDH_HQUERY,
    hcounters: Vec<winapi::PDH_HCOUNTER>,
}

impl PdhControler {
    fn new(path: Vec<&str>) -> Option<PdhControler> {
        pdh_open_query()
            .map(|q| {
                PdhControler {
                    hquery: q,
                    hcounters: path.into_iter()
                        .map(|p| pdh_add_counter(q, p))
                        .flat_map(|c| c)
                        .collect(),
                }
            }).ok()
    } 

    fn get_current_values(&self) -> Vec<PdhValue> {
        pdh_collect_query_data(self.hquery);
        self.hcounters.iter()
            .map(|&c| pdh_get_formatted_counter_value(c, PDH_FMT_DOUBLE))
            .flat_map(|v| v)
            .collect()
    } 
}

impl Drop for PdhControler {
    fn drop(&mut self) {
        pdh_close_query(self.hquery);        
    }
}

#[derive(Debug)]
enum PdhValue {
    LongLong(i64),
    Long(i32), 
    Double(f64), 
    Str(String)
}

fn pdh_open_query() -> Result<winapi::PDH_HQUERY, winapi::PDH_STATUS> {
    use std::ptr;
    let mut hquery = winapi::INVALID_HANDLE_VALUE;   
    unsafe {
        let ret = pdh::PdhOpenQueryW(ptr::null(), 0, &mut hquery);
        if winapi::winerror::SUCCEEDED(ret) {
            Ok(hquery)
        }else {
            Err(ret)
        }
    }    
}

fn pdh_collect_query_data(hquery: winapi::PDH_HQUERY) -> bool {
    unsafe {
        winapi::winerror::SUCCEEDED(pdh::PdhCollectQueryData(hquery))
    }
}

fn pdh_get_formatted_counter_value(hcounter: winapi::PDH_HCOUNTER, format: winapi::DWORD) -> Result<PdhValue, winapi::PDH_STATUS> {
    let mut s = winapi::PDH_FMT_COUNTERVALUE {CStatus: 0, largeValue: 0};
    unsafe {
        let mut devnul: winapi::DWORD = 0;
        let ret = pdh::PdhGetFormattedCounterValue(hcounter, format, &mut devnul, &mut s);
        if winapi::winerror::SUCCEEDED(ret) {
            Ok(to_value(s, format))
        }else {
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

fn pdh_close_query(hquery: PDH_HQUERY) -> bool {
    unsafe {
        winapi::winerror::SUCCEEDED(pdh::PdhCloseQuery(hquery))
    }    
}

fn pdh_add_counter(hquery: winapi::PDH_HQUERY, counter_path: &str) -> Result<winapi::PDH_HCOUNTER, winapi::PDH_STATUS> {
    let mut hcounter = winapi::INVALID_HANDLE_VALUE;   
    unsafe {
        let ret = pdh::PdhAddCounterW(hquery, to_wide_chars(counter_path).as_ptr(), 0, &mut hcounter);
        if winapi::winerror::SUCCEEDED(ret) {
            Ok(hcounter)
        }else {
            Err(ret)
        }
    }    
}

fn to_wide_chars(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(s).encode_wide().chain(Some(0).into_iter()).collect::<Vec<_>>()
}