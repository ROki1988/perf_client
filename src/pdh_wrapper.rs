extern crate winapi;
extern crate pdh;

use std;
use winapi::pdh::*;

#[derive(Debug)]
pub struct PdhController {
    hquery: winapi::PDH_HQUERY,
    hcounters: Vec<winapi::PDH_HCOUNTER>,
}

impl PdhController {
    pub fn new(path: &Vec<String>) -> Option<PdhController> {
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

    pub fn current_values(&self) -> Vec<PdhValue> {
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
pub struct PdhControllerIntoIterator {
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
pub enum PdhValue {
    LongLong(i64),
    Long(i32),
    Double(f64),
    Str(String),
}

#[derive(Debug, Default)]
pub struct PdhCounterPathElement {
    machine_name: Option<String>,
    object_name: String,
    parent_instance: Option<String>,
    instance_index: Option<u32>,
    instance_name: Option<String>,
    counter_name: String,
}

impl PdhCounterPathElement {
    pub fn new(o_name: String, c_name: String) -> PdhCounterPathElement {
        PdhCounterPathElement {
            object_name: o_name,
            counter_name: c_name,
            ..Default::default()
        }
    }
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

pub fn pdh_make_counter_path(element: &PdhCounterPathElement)
                             -> Result<String, winapi::PDH_STATUS> {
    use std::ptr;

    let mut object_name = to_wide_chars(element.object_name.as_str());
    let mut counter_name = to_wide_chars(element.counter_name.as_str());

    let machine_name = element.machine_name
        .clone()
        .map(|s| to_wide_chars(s.as_str()));
    let instance_name = element.instance_name
        .clone()
        .map(|s| to_wide_chars(s.as_str()));
    let parent_instance = element.parent_instance
        .clone()
        .map(|s| to_wide_chars(s.as_str()));

    let mut mut_element = PDH_COUNTER_PATH_ELEMENTS_W {
        szMachineName: machine_name.map_or(ptr::null_mut::<u16>(), |mut v| v.as_mut_ptr()),
        szObjectName: object_name.as_mut_ptr(),
        szCounterName: counter_name.as_mut_ptr(),
        szInstanceName: instance_name.map_or(ptr::null_mut::<u16>(), |mut v| v.as_mut_ptr()),
        szParentInstance: parent_instance.map_or(ptr::null_mut::<u16>(), |mut v| v.as_mut_ptr()),
        dwInstanceIndex: element.instance_index.unwrap_or(0),
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