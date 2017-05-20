#[cfg(windows)]
extern crate winapi;

extern crate widestring;
extern crate serde;

use winapi::um::pdh;
use winapi::um::handleapi;
use winapi::shared::minwindef::DWORD;
use winapi::um::winnt::{LONG};
use winapi::shared::winerror;
use widestring::*;


#[test]
fn test_pdh_controller_memory() {
    let pdhc = PdhController::new(vec![PdhCounterPathElement::new("Memory".to_string(),
                                                                  "Available Mbytes".to_string(),
                                                                  PdhCounterPathElementOptions {
                                                                      ..Default::default()
                                                                  })])
        .unwrap();
    assert!(pdhc.iter().next().is_some());
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

    assert!(pdhc.iter().next().is_some());
}

#[test]
fn test_pdh_controller_processor() {
    let pdhc = PdhController::new(vec![PdhCounterPathElement::new("Processor".to_string(),
                                                                  "% User Time".to_string(),
                                                                  PdhCounterPathElementOptions {
                                                                      instance_name: Some("_Total"
                                                                          .to_string()),
                                                                      ..Default::default()
                                                                  })])
        .unwrap();
    assert!(pdhc.iter().next().is_some());
}

#[derive(Debug)]
pub struct PdhCollectionItem {
    element: PdhCounterPathElement,
    hcounter: pdh::PDH_HCOUNTER,
}

#[derive(Debug)]
pub struct PdhController {
    hquery: pdh::PDH_HQUERY,
    items: Vec<PdhCollectionItem>,
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub enum PdhCollectError {
    PdhStatus(pdh::PDH_STATUS),
    WinError(LONG),
    Other(String),
}

impl PdhController {
    pub fn new(path: Vec<PdhCounterPathElement>) -> Option<PdhController> {
        pdh_open_query()
            .map(|q| {
                let cs = path.into_iter()
                    .flat_map(|e| {
                        pdh_add_counter(q, &e).map(|c| {
                            PdhCollectionItem {
                                element: e,
                                hcounter: c,
                            }
                        })
                    })
                    .collect::<Vec<_>>();
                pdh_collect_query_data(q);
                PdhController {
                    hquery: q,
                    items: cs,
                }
            })
            .ok()
    }

    pub fn iter(&self) -> PdhControllerIterator {
        PdhControllerIterator {
            pdhc: &self,
            index: 0,
        }
    }
}

impl Drop for PdhController {
    fn drop(&mut self) {
        pdh_close_query(self.hquery).expect("Can't Close PdhQyery");
    }
}

#[derive(Debug)]
pub struct PdhControllerIterator<'a> {
    pdhc: &'a PdhController,
    index: usize,
}

impl<'a> Iterator for PdhControllerIterator<'a> {
    type Item = PdhCollectValue;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == 0 {
            pdh_collect_query_data(self.pdhc.hquery);
        }

        let item = self.pdhc
            .items
            .get(self.index)
            .iter()
            .flat_map(|c| {
                pdh_get_formatted_counter_value(c.hcounter, pdh::PDH_FMT_DOUBLE)
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


#[derive(Debug, Clone)]
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


#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub enum PdhValue {
    LongLong(i64),
    Long(i32),
    Double(f64),
    Str(String),
}

impl serde::ser::Serialize for PdhValue {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: serde::ser::Serializer
    {
        match *self {
            PdhValue::LongLong(ref ll) => serializer.serialize_i64(*ll),
            PdhValue::Long(ref l) => serializer.serialize_i64(*l as i64),
            PdhValue::Double(ref d) => serializer.serialize_f64(*d),
            PdhValue::Str(ref s) => serializer.serialize_str(s),
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct PdhCounterPathElement {
    pub object_name: String,
    pub counter_name: String,
    pub options: PdhCounterPathElementOptions,
}

#[derive(Debug, Default, Clone, Deserialize)]
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

fn pdh_open_query() -> Result<pdh::PDH_HQUERY, PdhCollectError> {
    use std::ptr;
    let mut hquery = handleapi::INVALID_HANDLE_VALUE;
    let ret = unsafe { pdh::PdhOpenQueryW(ptr::null(), 0, &mut hquery) };

    if winerror::SUCCEEDED(ret) {
        Ok(hquery)
    } else {
        Err(PdhCollectError::PdhStatus(ret))
    }
}

fn pdh_collect_query_data(hquery: pdh::PDH_HQUERY) -> bool {
    unsafe { winerror::SUCCEEDED(pdh::PdhCollectQueryData(hquery)) }
}

fn pdh_get_formatted_counter_value(hcounter: pdh::PDH_HCOUNTER,
                                   format: DWORD)
                                   -> Result<PdhValue, PdhCollectError> {
    use std::ptr;
    use std::mem;

    let mut s: pdh::PDH_FMT_COUNTERVALUE = unsafe { mem::zeroed() };

    let ret = unsafe {
        pdh::PdhGetFormattedCounterValue(hcounter, format, ptr::null_mut::<u32>(), &mut s)
    };

    if winerror::SUCCEEDED(ret) {
        Ok(to_value(&s, format))
    } else {
        Err(PdhCollectError::PdhStatus(ret))
    }
}

fn to_value(s: &pdh::PDH_FMT_COUNTERVALUE, format: DWORD) -> PdhValue {
    unsafe {
        match format {
            pdh::PDH_FMT_DOUBLE => PdhValue::Double(*s.doubleValue()),
            pdh::PDH_FMT_LONG => PdhValue::Long(*s.longValue()),
            pdh::PDH_FMT_LARGE => PdhValue::LongLong(*s.largeValue()),
            _ => PdhValue::Long(*s.longValue()),
        }
    }
}

fn pdh_close_query(hquery: pdh::PDH_HQUERY) -> Result<(), PdhCollectError> {
    let ret = unsafe { pdh::PdhCloseQuery(hquery) };

    if winerror::SUCCEEDED(ret) {
        Ok(())
    } else {
        Err(PdhCollectError::PdhStatus(ret))
    }
}

fn pdh_add_counter(hquery: pdh::PDH_HQUERY,
                   counter_element: &PdhCounterPathElement)
                   -> Result<pdh::PDH_HCOUNTER, PdhCollectError> {
    let mut hcounter = handleapi::INVALID_HANDLE_VALUE;
    let path = pdh_make_counter_path(counter_element)?;
    let ret = unsafe { pdh::PdhAddCounterW(hquery, path.as_ptr(), 0, &mut hcounter) };
    if winerror::SUCCEEDED(ret) {
        Ok(hcounter)
    } else {
        Err(PdhCollectError::PdhStatus(ret))
    }
}

pub fn pdh_make_counter_path(element: &PdhCounterPathElement) -> Result<Vec<u16>, PdhCollectError> {
    use std::ptr;
    use winapi::shared::winerror;

    let to_wide_str = |s: &str| {
        WideCString::from_str(s)
            .map(|ws| ws.into_vec())
            .or(Err(PdhCollectError::WinError(winerror::ERROR_BAD_ARGUMENTS as i32)))
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

    let mut mut_element = pdh::PDH_COUNTER_PATH_ELEMENTS_W {
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

    if winerror::SUCCEEDED(ret) {
        let valid = unsafe { pdh::PdhValidatePathW(buff.as_mut_ptr()) };
        if winerror::SUCCEEDED(valid) {
            Ok(buff)
        } else {
            Err(PdhCollectError::PdhStatus(valid))
        }
    } else {
        Err(PdhCollectError::PdhStatus(ret))
    }
}

#[test]
fn test_pdh_make_counter_path_memory() {
    let element = PdhCounterPathElement::new("Memory".to_string(),
                                             "Available Mbytes".to_string(),
                                             PdhCounterPathElementOptions { ..Default::default() });
    let v = pdh_make_counter_path(&element).unwrap();

    assert_eq!(WideCString::from_vec_with_nul(v).unwrap(),
               WideCString::from_str("\\Memory\\Available Mbytes").unwrap());
}

#[test]
fn test_pdh_make_counter_path_process() {
    let element = PdhCounterPathElement::new("Process".to_string(),
                                             "% Processor Time".to_string(),
                                             PdhCounterPathElementOptions {
                                                 instance_name: Some("code".to_string()),
                                                 ..Default::default()
                                             });
    let v = pdh_make_counter_path(&element).unwrap();

    assert_eq!(WideCString::from_vec_with_nul(v).unwrap(),
               WideCString::from_str("\\Process(code)\\% Processor Time").unwrap());
}

fn pdh_get_counter_path_buff_size(element: pdh::PPDH_COUNTER_PATH_ELEMENTS_W)
                                  -> Result<DWORD, PdhCollectError> {
    use std::ptr;

    let mut buff_size = 0;

    let status =
        unsafe { pdh::PdhMakeCounterPathW(element, ptr::null_mut::<u16>(), &mut buff_size, 0) };

    if status == 0x800007D2 {
        Ok(buff_size)
    } else {
        Err(PdhCollectError::PdhStatus(status))
    }
}

fn to_wide_chars(s: &str) -> Vec<u16> {
    WideCString::from_str(s).map(|ws| ws.into_vec_with_nul()).unwrap()
}
