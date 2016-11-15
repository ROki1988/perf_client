#[cfg(windows)]
extern crate winapi;
#[cfg(windows)]
extern crate pdh;
#[cfg(windows)]
extern crate widestring;
extern crate serde;
extern crate rustc_serialize;

use winapi::pdh::*;
use widestring::*;
use serde::ser;


#[test]
fn test_pdh_controller_memory() {
    let pdhc = PdhController::new(vec![PdhCounterPathElement::new("Memory".to_string(),
                                                                  "Available Mbytes".to_string(),
                                                                  PdhCounterPathElementOptions {
                                                                      ..Default::default()
                                                                  })])
        .unwrap();
    debug_assert!(pdhc.into_iter().next().is_some());
}

#[test]
fn test_pdh_controller_process() {
    let pdhc = PdhController::new(vec![PdhCounterPathElement::new("Process".to_string(),
                                                                  "Thread Count".to_string(),
                                                                  PdhCounterPathElementOptions {
                                                                      instance_name: Some("explorer"
                                                                          .to_string()),
                                                                      ..Default::default()
                                                                  })])
        .unwrap();

    debug_assert!(pdhc.into_iter().next().is_some());
}

#[test]
fn test_pdh_controller_processor() {
    let pdhc = PdhController::new(vec![PdhCounterPathElement::new("Processor".to_string(),
                                                                  "% Processor Time".to_string(),
                                                                  PdhCounterPathElementOptions {
                                                                      instance_name: Some("_Total"
                                                                          .to_string()),
                                                                      ..Default::default()
                                                                  })])
        .unwrap();


    debug_assert!(pdhc.into_iter().next().is_some());
}

#[derive(Debug)]
pub struct PdhCollectionItem {
    element: PdhCounterPathElement,
    hcounter: winapi::PDH_HCOUNTER,
}

#[derive(Debug)]
pub struct PdhController {
    hquery: winapi::PDH_HQUERY,
    items: Vec<PdhCollectionItem>,
}

impl PdhController {
    pub fn new(path: Vec<PdhCounterPathElement>) -> Option<PdhController> {
        pdh_open_query()
            .map(|q| {
                let cs = path.into_iter()
                    .flat_map(|e| {
                        pdh_make_counter_path(&e)
                            .and_then(|p| pdh_add_counter(q, p.as_str()))
                            .map(|c| {
                                PdhCollectionItem {
                                    element: e,
                                    hcounter: c,
                                }
                            })
                    })
                    .collect::<Vec<_>>();
                PdhController {
                    hquery: q,
                    items: cs,
                }
            })
            .ok()
    }
}

impl IntoIterator for PdhController {
    type IntoIter = PdhControllerIntoIterator;
    type Item = PdhCollectValue;

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
    type Item = <PdhController as IntoIterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == 0 {
            pdh_collect_query_data(self.pdhc.hquery);
        }

        let item = self.pdhc
            .items
            .get(self.index)
            .iter()
            .flat_map(|c| {
                pdh_get_formatted_counter_value(c.hcounter, PDH_FMT_DOUBLE)
                    .map(|v| PdhCollectValue::new(&c.element, v))
            })
            .last();
        self.index += 1;
        item
    }
}

trait CollectionValue {
    fn new(e: &PdhCounterPathElement, v: PdhValue) -> Self;
}


#[derive(Debug)]
pub struct PdhCollectValue {
    pub element: PdhCounterPathElement,
    pub value: PdhValue,
}

impl CollectionValue for PdhCollectValue {
    fn new(e: &PdhCounterPathElement, v: PdhValue) -> Self {
        PdhCollectValue {
            element: e.clone(),
            value: v,
        }
    }
}

impl ToString for PdhCollectValue {
    fn to_string(&self) -> String {
        format!("{} \t {}", self.element.to_string(), self.value.to_string())
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


#[derive(Debug, Default, Clone, RustcDecodable)]
pub struct PdhCounterPathElement {
    pub object_name: String,
    pub counter_name: String,
    pub options: PdhCounterPathElementOptions,
}

#[derive(Debug, Default, Clone, RustcDecodable)]
pub struct PdhCounterPathElementOptions {
    pub machine_name: Option<String>,
    pub parent_instance: Option<String>,
    pub instance_index: Option<u32>,
    pub instance_name: Option<String>,
}

impl PdhCounterPathElement {
    pub fn new(o_name: String,
               c_name: String,
               ops: PdhCounterPathElementOptions)
               -> PdhCounterPathElement {
        PdhCounterPathElement {
            object_name: o_name,
            counter_name: c_name,
            options: ops,
        }
    }
}

fn pdh_open_query() -> Result<winapi::PDH_HQUERY, winapi::PDH_STATUS> {
    use std::ptr;
    let mut hquery = winapi::INVALID_HANDLE_VALUE;
    let ret = unsafe { pdh::PdhOpenQueryW(ptr::null(), 0, &mut hquery) };

    if winapi::winerror::SUCCEEDED(ret) {
        Ok(hquery)
    } else {
        Err(ret)
    }
}

fn pdh_collect_query_data(hquery: winapi::PDH_HQUERY) -> bool {
    unsafe { winapi::winerror::SUCCEEDED(pdh::PdhCollectQueryData(hquery)) }
}

fn pdh_get_formatted_counter_value(hcounter: winapi::PDH_HCOUNTER,
                                   format: winapi::DWORD)
                                   -> Result<PdhValue, winapi::PDH_STATUS> {
    use std::ptr;

    let mut s = winapi::PDH_FMT_COUNTERVALUE {
        CStatus: 0,
        largeValue: 0,
    };

    let ret = unsafe {
        pdh::PdhGetFormattedCounterValue(hcounter, format, ptr::null_mut::<u32>(), &mut s)
    };

    if winapi::winerror::SUCCEEDED(ret) {
        Ok(to_value(&s, format))
    } else {
        Err(ret)
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
    let ret = unsafe { pdh::PdhCloseQuery(hquery) };

    if winapi::winerror::SUCCEEDED(ret) {
        Ok(())
    } else {
        Err(ret)
    }
}

fn pdh_add_counter(hquery: winapi::PDH_HQUERY,
                   counter_path: &str)
                   -> Result<winapi::PDH_HCOUNTER, winapi::PDH_STATUS> {
    let mut hcounter = winapi::INVALID_HANDLE_VALUE;
    let ret = unsafe {
        pdh::PdhAddCounterW(hquery,
                            to_wide_chars(counter_path).as_ptr(),
                            0,
                            &mut hcounter)
    };
    if winapi::winerror::SUCCEEDED(ret) {
        Ok(hcounter)
    } else {
        Err(ret)
    }
}

pub fn pdh_make_counter_path(element: &PdhCounterPathElement)
                             -> Result<String, winapi::PDH_STATUS> {
    use std::ptr;
    use winapi::winerror;

    let to_wide_str = |s: &str| {
        WideCString::from_str(s)
            .map(|ws| ws.into_vec())
            .map_err(|e| winerror::ERROR_BAD_ARGUMENTS as i32)
    };
    let mut object_name = to_wide_str(element.object_name.as_str())?;
    let mut counter_name = to_wide_str(element.counter_name.as_str())?;

    let machine_name = element.options
        .machine_name
        .clone()
        .map(|s| to_wide_chars(s.as_str()));
    let instance_name = element.options
        .instance_name
        .clone()
        .map(|s| to_wide_chars(s.as_str()));
    let parent_instance = element.options
        .parent_instance
        .clone()
        .map(|s| to_wide_chars(s.as_str()));

    let mut mut_element = PDH_COUNTER_PATH_ELEMENTS_W {
        szMachineName: machine_name.map_or(ptr::null_mut::<u16>(), |mut v| v.as_mut_ptr()),
        szObjectName: object_name.as_mut_ptr(),
        szCounterName: counter_name.as_mut_ptr(),
        szInstanceName: instance_name.map_or(ptr::null_mut::<u16>(), |mut v| v.as_mut_ptr()),
        szParentInstance: parent_instance.map_or(ptr::null_mut::<u16>(), |mut v| v.as_mut_ptr()),
        dwInstanceIndex: element.options.instance_index.unwrap_or(0),
    };

    let mut buff_size = pdh_get_counter_path_buff_size(&mut mut_element)?;
    let mut buff = vec![ 0u16; (buff_size + 1) as usize ];

    let ret =
        unsafe { pdh::PdhMakeCounterPathW(&mut mut_element, buff.as_mut_ptr(), &mut buff_size, 0) };

    if winapi::winerror::SUCCEEDED(ret) {
        buff.truncate(buff_size as usize);
        Ok(WideString::from_vec(buff).to_string_lossy())
    } else {
        Err(ret)
    }
}

#[test]
fn test_pdh_make_counter_path_memory() {
    let element = PdhCounterPathElement::new("Memory".to_string(),
                                             "Available Mbytes".to_string(),
                                             PdhCounterPathElementOptions { ..Default::default() });
    let v = pdh_make_counter_path(&element);

    assert_eq!(v, Ok("\\Memory\\Available Mbytes\u{0}".to_string()));
}

#[test]
fn test_pdh_make_counter_path_process() {
    let element = PdhCounterPathElement::new("Process".to_string(),
                                             "% Processor Time".to_string(),
                                             PdhCounterPathElementOptions {
                                                 instance_name: Some("code".to_string()),
                                                 ..Default::default()
                                             });
    let v = pdh_make_counter_path(&element);

    assert_eq!(v, Ok("\\Process(code)\\% Processor Time\u{0}".to_string()));
}

fn pdh_get_counter_path_buff_size(element: PPDH_COUNTER_PATH_ELEMENTS_W)
                                  -> Result<winapi::DWORD, winapi::PDH_STATUS> {
    use std::ptr;

    let mut buff_size = 0;

    let status =
        unsafe { pdh::PdhMakeCounterPathW(element, ptr::null_mut::<u16>(), &mut buff_size, 0) };

    if status == 0x800007D2 {
        Ok(buff_size)
    } else {
        Err(status)
    }
}

fn to_wide_chars(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    OsStr::new(s).encode_wide().chain(Some(0).into_iter()).collect::<Vec<_>>()
}
