use std::thread;

use ringbuf::{SharedRb};
use interoptopus::patterns::string::*;
use interoptopus::{ffi_function, function, Inventory, InventoryBuilder};

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
    thread::spawn(move || {
        loop {
            data_queue_tx.push(String::from("Hello World!\0")).unwrap();
            
        }
    }).join().unwrap();
    AsciiPointer::from_slice_with_nul(data_queue_rx.pop().unwrap().as_bytes()).unwrap()
}

#[no_mangle]
#[ffi_function]
pub extern "C" fn t_hw_print<'a>(to_print: AsciiPointer<'static >) {
    let tp = to_print.as_str().map_err(|e| e.to_string());
    match tp {
        Ok(r) => println!("{}", r),
        Err(e) => println!("{}", e)
    }
    
}



// Define our FFI interface as `ffi_inventory` containing
// a single function `my_function`. Types are inferred.
pub fn ffi_inventory() -> Inventory {
    InventoryBuilder::new()
        .register(function!(nt_hello_world))
        .register(function!(t_hw))
        .register(function!(t_hw_print))
        .inventory()
}
