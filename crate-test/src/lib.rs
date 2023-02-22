#[macro_use]
extern crate lazy_static;
extern crate futures;

use chrono::{DateTime, Local};
use std::collections::HashMap;
use std::io;
use std::mem::MaybeUninit;
use std::str;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::SystemTime;
use std::ffi::CStr;
use std::ffi::c_char;

use bytes::BytesMut;
use futures::stream::StreamExt;
use regex::Regex;
use ringbuf::{Consumer, HeapRb, SharedRb};
use interoptopus::patterns::string::*;
use interoptopus::{ffi_function, ffi_type, function, Inventory, InventoryBuilder};

lazy_static! {
    static ref STOP: AtomicBool = AtomicBool::new(false);
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
    let data_queue = HeapRb::<String>::new(255);
    // Recuperer Producteur et Consommateur
    let (mut data_queue_tx, mut data_queue_rx) = data_queue.split();
    // Envoi du consommateur dans le thread pour le traitement
    thread::spawn(move || {
        loop {
            data_queue_tx.push(String::from("Hello World!\0")).unwrap();
            
        }
    }).join().unwrap();
    AsciiPointer::from_slice_with_nul(data_queue_rx.pop().unwrap().as_bytes()).unwrap()
}

pub extern "C" fn t_hw_print<'a>() -> AsciiPointer<'a> {

}



// Define our FFI interface as `ffi_inventory` containing
// a single function `my_function`. Types are inferred.
pub fn ffi_inventory() -> Inventory {
    InventoryBuilder::new()
        .register(function!(nt_hello_world))
        .register(function!(t_hw))
        .inventory()
}
