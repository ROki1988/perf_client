extern crate winapi;
extern crate pdh;

use winapi::pdh::*;

fn main() {
    let element_list = vec![PdhCounterPathElement {
                                MachineName: None,
                                ObjectName: String::from("Memory"),
                                ParentInstance: None,
                                InstanceIndex: None,
                                InstanceName: None,
                                CounterName: String::from("Available Mbytes"),
                            }];

    let path_list = element_list.into_iter()
        .filter_map(|e| pdh_make_counter_path(&e).ok())
        .collect::<Vec<_>>();

    let pdhc = PdhController::new(&path_list).expect("Can't create Metrics Collector");
    println!("{:?}",
             pdhc.into_iter().map(|v| pdh_value2str(v)).collect::<Vec<_>>());
}

fn pdh_value2str(v: PdhValue) -> String {
    match v {
        PdhValue::LongLong(ll) => format!("{:.3}", ll),
        PdhValue::Long(l) => format!("{:.3}", l),
        PdhValue::Double(d) => format!("{:.3}", d),
        PdhValue::Str(s) => s,
    }
}

#[derive(Debug)]
struct PdhController {
    hquery: winapi::PDH_HQUERY,
    hcounters: Vec<winapi::PDH_HCOUNTER>,
}

impl PdhController {
    fn new(path: &Vec<String>) -> Option<PdhController> {
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

#[derive(Debug)]
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
        self.index += 1;
        match v {
            Some(Some(a)) => Some(a),
            _ => None,
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
enum PdhValue {
    LongLong(i64),
    Long(i32),
    Double(f64),
    Str(String),
}

#[derive(Debug)]
struct PdhCounterPathElement {
    MachineName: Option<String>,
    ObjectName: String,
    ParentInstance: Option<String>,
    InstanceIndex: Option<u32>,
    InstanceName: Option<String>,
    CounterName: String,
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
            Ok(to_value(&s, format))
        } else {
            Err(ret)
        }
    }
}

fn to_value(s: &PDH_FMT_COUNTERVALUE, format: winapi::DWORD) -> PdhValue {
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

fn pdh_make_counter_path(element: &PdhCounterPathElement) -> Result<String, winapi::PDH_STATUS> {
    use std::ptr;

    let mut ObjectName = to_wide_chars(element.ObjectName.as_str());
    let mut CounterName = to_wide_chars(element.CounterName.as_str());

    let mut MachineName = element.MachineName
        .clone()
        .map(|s| to_wide_chars(s.as_str()));
    let mut InstanceName = element.InstanceName
        .clone()
        .map(|s| to_wide_chars(s.as_str()));
    let mut ParentInstance = element.ParentInstance
        .clone()
        .map(|s| to_wide_chars(s.as_str()));

    let mut mut_element = PDH_COUNTER_PATH_ELEMENTS_W {
        szMachineName: MachineName.map_or(ptr::null_mut::<u16>(), |mut v| v.as_mut_ptr()),
        szObjectName: ObjectName.as_mut_ptr(),
        szCounterName: CounterName.as_mut_ptr(),
        szInstanceName: InstanceName.map_or(ptr::null_mut::<u16>(), |mut v| v.as_mut_ptr()),
        szParentInstance: ParentInstance.map_or(ptr::null_mut::<u16>(), |mut v| v.as_mut_ptr()),
        dwInstanceIndex: element.InstanceIndex.unwrap_or(0),
    };

    let mut buff_size = try!(pdh_get_counter_path_buff_size(&mut mut_element));
    let mut buff = vec![ 0u16; (buff_size + 1) as usize ];

    unsafe {
        let ret = pdh::PdhMakeCounterPathW(&mut mut_element, buff.as_mut_ptr(), &mut buff_size, 0);
        if winapi::winerror::SUCCEEDED(ret) {
            buff.truncate(buff_size as usize);
            Ok(from_wide_ptr(buff.as_ptr()))
        } else {
            Err(ret)
        }
    }
}

fn pdh_get_counter_path_buff_size(element: PPDH_COUNTER_PATH_ELEMENTS_W)
                                  -> Result<winapi::DWORD, winapi::PDH_STATUS> {
    use std::ptr;

    unsafe {
        let mut buff_size = 0;
        let status = pdh::PdhMakeCounterPathW(element, ptr::null_mut::<u16>(), &mut buff_size, 0);
        if status == 0x800007D2 {
            Ok(buff_size)
        } else {
            Err(status)
        }
    }
}

fn to_wide_chars(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    OsStr::new(s).encode_wide().chain(Some(0).into_iter()).collect::<Vec<_>>()
}

fn from_wide_ptr(ptr: *const u16) -> String {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    unsafe {
        assert!(!ptr.is_null());
        let len = (0..std::isize::MAX).position(|i| *ptr.offset(i) == 0).unwrap();
        let slice = std::slice::from_raw_parts(ptr, len);
        OsString::from_wide(slice).to_string_lossy().into_owned()
    }
}
