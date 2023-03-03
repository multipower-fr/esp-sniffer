#![no_main]
use ::core::{ptr, slice};
use std::ffi::*;
use std::thread;

use interoptopus::patterns::string::*;
use interoptopus::{ffi_function, function, Inventory, InventoryBuilder};
use rifgen::rifgen_attr::*;
use ringbuf::SharedRb;

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Data {
    mac: String,
    rssi: i32,
    channels: u32,
    ssids: String,
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataRaw {
    mac: *const c_char,
    rssi: i32,
    channels: *mut u16,
    ssids: *const c_char,
}

#[repr(C)]
pub struct FFIBoxedSlice {
    ptr: *mut DataRaw,
    len: usize, // number of elems
}

#[no_mangle]
#[ffi_function]
pub extern "C" fn nt_hello_world<'a>() -> AsciiPointer<'a> {
    let s = String::from("Hello World\0");
    AsciiPointer::from_slice_with_nul(s.as_bytes()).unwrap()
}

#[no_mangle]
#[ffi_function]
pub extern "C" fn t_hw<'a>() -> AsciiPointer<'a> {
    // FIFO queue
    let data_queue = SharedRb::<String, Vec<_>>::new(255);
    // Recuperer Producteur et Consommateur
    let (mut data_queue_tx, mut data_queue_rx) = data_queue.split();
    // Envoi du consommateur dans le thread pour le traitement
    thread::spawn(move || loop {
        data_queue_tx.push(String::from("Hello World!\0")).unwrap();
    })
    .join()
    .unwrap();
    AsciiPointer::from_slice_with_nul(data_queue_rx.pop().unwrap().as_bytes()).unwrap()
}

#[no_mangle]
#[ffi_function]
pub extern "C" fn t_hw_print<'a>(to_print: AsciiPointer<'static>) {
    let tp = to_print.as_str().map_err(|e| e.to_string());
    match tp {
        Ok(r) => println!("{}", r),
        Err(e) => println!("{}", e),
    }
}

#[no_mangle]
pub extern "C" fn test_struct() -> FFIBoxedSlice {
    let mut vec_structs: Vec<DataRaw> = Vec::new();
    for _ in 1..=2 {
        let mut ssids = String::from("a").to_owned();
        let ssid_to_add = String::from("q\0").to_owned();
        ssids.push_str(&ssid_to_add);
        let mac = CString::new("DC-4B-04-19-12-E7\0").unwrap();
        let ssid_cstring = CString::new(ssids).unwrap();
        let mut chans = vec![1, 2];
        let chans_ptr = chans.as_mut_ptr();
        std::mem::forget(chans_ptr);
        let data = DataRaw {
            mac: mac.as_ptr(),
            channels: chans_ptr,
            rssi: -3,
            ssids: ssid_cstring.as_ptr(),
        };
        vec_structs.push(data);
    }
    vec_to_ffi(vec_structs)
}

#[no_mangle]
pub extern "C" fn test_struct_2() -> FFIBoxedSlice {
    let mut vec_structs: Vec<DataRaw> = Vec::new();
    for _ in 1..=2 {
        let mut ssids = String::from("a").to_owned();
        let ssid_to_add = String::from("q").to_owned();
        ssids.push_str(&ssid_to_add);
        let mac_str = String::from("DC-4B-04-19-12-E7\0").into_bytes();
        let mac = CString::from_vec_with_nul(mac_str).unwrap();
        let ssid_cstring = CString::new(ssids).unwrap();
        let mut chans = vec![1, 2];
        let chans_ptr = chans.as_mut_ptr();
        std::mem::forget(chans_ptr);
        let data = DataRaw {
            mac: mac.as_ptr(),
            channels: chans_ptr,
            rssi: -3,
            ssids: ssid_cstring.as_ptr(),
        };
        vec_structs.push(data);
    }
    vec_to_ffi(vec_structs)
}

// Helper (internal) function
fn vec_to_ffi(v: Vec<DataRaw>) -> FFIBoxedSlice {
    // Going from Vec<_> to Box<[_]> just drops the (extra) `capacity`
    let boxed_slice: Box<[DataRaw]> = v.into_boxed_slice();
    let len = boxed_slice.len();
    let fat_ptr: *mut [DataRaw] = Box::into_raw(boxed_slice);
    let slim_ptr: *mut DataRaw = fat_ptr as _;
    FFIBoxedSlice { ptr: slim_ptr, len }
}

#[no_mangle]
pub unsafe extern "C" fn free_boxed_slice(FFIBoxedSlice { ptr, len }: FFIBoxedSlice) {
    if ptr.is_null() {
        eprintln!("free_boxed_slice() errored: got NULL ptr!");
        ::std::process::abort();
    }
    let slice: &mut [DataRaw] = slice::from_raw_parts_mut(ptr, len);
    drop(Box::from_raw(slice));
}

// Define our FFI interface as `ffi_inventory` containing
// a single function `my_function`. Types are inferred.
pub fn ffi_inventory() -> Inventory {
    InventoryBuilder::new()
        .register(function!(t_hw))
        .register(function!(t_hw_print))
        .inventory()
}
