#[macro_use]
extern crate lazy_static;
extern crate futures;

use chrono::{DateTime, Local};
use interoptopus::patterns::string::*;
use interoptopus::{ffi_function, function, Inventory, InventoryBuilder};
use std::collections::HashMap;
use std::fmt::Write;
use std::io;
use std::mem::MaybeUninit;
use std::str;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::SystemTime;
use std::ffi::*;
use ::core::{ptr, slice};

use bytes::BytesMut;
use futures::stream::StreamExt;
use regex::Regex;
use ringbuf::{Consumer, HeapRb, SharedRb};
use tokio_serial::SerialPortBuilderExt;
use tokio_util::codec::{Decoder, Encoder};

lazy_static! {
    // Listes des adresses MAC
    static ref MACS: Arc<Mutex<Vec<String>>> = {
        let macs: Vec<String> = Vec::new();
        Arc::new(Mutex::new(macs))
    };
    // HashMap avec les canaux ou certaines MAC sont visibles
    static ref CHANNELS: Arc<Mutex<HashMap<String, Vec<u32>>>> = {
        let cha = HashMap::new();
        Arc::new(Mutex::new(cha))
    };
    // HashMap avec les SSIDs récupérés
    static ref SSIDS: Arc<Mutex<HashMap<String, Vec<String>>>> = {
        let s = HashMap::new();
        Arc::new(Mutex::new(s))
    };
    // HashMap avec le dernier RSSI
    static ref RSSIS: Arc<Mutex<HashMap<String, i32>>> = {
        let r = HashMap::new();
        Arc::new(Mutex::new(r))
    };
    // HashMap avec les timestamp ou les addresses MAC sont vues en dernier
    static ref LAST_SEEN: Arc<Mutex<HashMap<String, SystemTime>>> = {
        let ts = HashMap::new();
        Arc::new(Mutex::new(ts))
    };
    static ref MAC_REGEX: Regex =
        Regex::new(r"^([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})$").unwrap();
    static ref STOP: AtomicBool = AtomicBool::new(false);
}

// Structure utilisée pour scinder les informations reçues par lignes
struct LineCodec;

impl Decoder for LineCodec {
    type Item = String;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let newline = src.as_ref().iter().position(|b| *b == b'\n');
        if let Some(n) = newline {
            let line = src.split_to(n + 1);
            return match str::from_utf8(line.as_ref()) {
                Ok(s) => Ok(Some(s.to_string())),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other, "Invalid String")),
            };
        }
        Ok(None)
    }
}

impl Encoder<String> for LineCodec {
    type Error = io::Error;

    fn encode(&mut self, _item: String, _dst: &mut BytesMut) -> Result<(), Self::Error> {
        Ok(())
    }
}
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Data {
    mac: String,
    rssi: i32,
    channels: Vec<u32>,
    ssids: String
}

#[repr(C)]
pub struct FFIBoxedSlice {
    ptr: *mut Data,
    len: usize, // number of elems
}


#[tokio::main]
async fn serial_port(port_name: String) -> tokio_serial::Result<()> {
    let port = tokio_serial::new(port_name, 115_200).open_native_async()?;
    let mut reader = LineCodec.framed(port);
    // FIFO queue
    let data_queue = HeapRb::<String>::new(255);
    // Recuperer Producteur et Consommateur
    let (mut data_queue_tx, data_queue_rx) = data_queue.split();
    // Envoi du consommateur dans le thread pour le traitement
    thread::spawn(move || {
        parse_str(data_queue_rx);
    });
    while let Some(line_result) = reader.next().await {
        let line = line_result.expect("Failed to read line");
        if STOP.load(Ordering::SeqCst) {
            STOP.store(false, Ordering::SeqCst);
            break;
        }
        // Si la ligne n'est pas mauvaise, la push sur le FIFO
        data_queue_tx.push(line).unwrap();
    }
    Ok(())
}

#[no_mangle]
#[ffi_function]
pub extern "C" fn start(tty_no: u32) -> u8 {
    let mut port_name: String = "COM".to_owned();
    STOP.store(false, Ordering::SeqCst);
    port_name.push_str(tty_no.to_string().as_str());
    thread::spawn(move || {
        serial_port(port_name).unwrap();
    });
    0
}

#[no_mangle]
#[ffi_function]
pub extern "C" fn stop() -> u8 {
    STOP.store(true, Ordering::SeqCst);
    0
}

#[allow(clippy::type_complexity)]
fn parse_str(mut data_queue_rx: Consumer<String, Arc<SharedRb<String, Vec<MaybeUninit<String>>>>>) {
    loop {
        while data_queue_rx.is_empty() {
            // Ne rien faire si la queue est vide
        }
        // Recupere un element de la FIFO
        let frame: String = data_queue_rx.pop().unwrap();
        // Clean la trame en enlevant tout les caractères spéciaux qui nous
        let cleaned_frame: String = frame.replace(&['\u{2}', '\u{3}', '\r', '\n'][..], "");
        // Split au niveau des caracteres de controle
        let splitted_frame: Vec<_> = cleaned_frame.split('\x1F').collect();
        // Check suivant si le channel est a un chiffre ou a deux chiffres
        let channel: u32 = (splitted_frame[0]).parse::<u32>().unwrap_or(0);
        let mac_address: String = if MAC_REGEX.is_match(splitted_frame[1]) {
            splitted_frame[1].to_string()
        } else {
            // Si l'addresse MAC est invalide
            "".to_string()
        };
        // Récupère le RSSI
        let rssi: String = splitted_frame[2].to_string();
        // Vérifie que le SSID n'est pas le RSSI
        let ssid: String = if splitted_frame[splitted_frame.len() - 2] != rssi {
            splitted_frame[splitted_frame.len() - 2].to_string()
        } else {
            splitted_frame[splitted_frame.len() - 1].to_string()
        };
        store(channel, mac_address, rssi, ssid);
    }
}

fn store(channel: u32, mac_address: String, rssi: String, ssid: String) {
    // Récupérer le lock sur les hashmaps
    let mut mac_table = MACS.lock().unwrap();
    // Ajouter a la liste des Adresses MAC connues si non dupliquées
    if !mac_table.contains(&mac_address) {
        mac_table.push(mac_address.clone())
    }

    // Mets a jour le timestamp
    let mut timestamp_table = LAST_SEEN.lock().unwrap();
    timestamp_table
        .entry(mac_address.clone())
        .and_modify(|ts_tmp| *ts_tmp = SystemTime::now())
        .or_insert(SystemTime::now());

    let mut channel_table = CHANNELS.lock().unwrap();
    // S'assurer de la présence du vecteur dans la table
    if let Some(tmp_channel) = channel_table.get(&mac_address) {
        // Déduplication
        if !tmp_channel.contains(&channel) {
            channel_table
                .entry(mac_address.clone())
                .or_insert_with(Vec::new)
                .push(channel);
        }
    } else {
        // Le créer si pas présent
        channel_table
            .entry(mac_address.clone())
            .or_insert_with(Vec::new)
            .push(channel);
    }

    // Ne rien faire si le SSID est vide
    if !ssid.is_empty() {
        // Récupérer le lock sur la table des SSIDs
        let mut ssid_table = SSIDS.lock().unwrap();
        // S'assurer de la présence du vecteur dans la table
        if let Some(tmp_ssid) = ssid_table.get(&mac_address) {
            // Déduplication
            if !tmp_ssid.contains(&ssid) {
                ssid_table
                    .entry(mac_address.clone())
                    .or_insert_with(Vec::new)
                    .push(ssid);
            }
        }
        // Le créer si pas présent
        else {
            ssid_table
                .entry(mac_address.clone())
                .or_insert_with(Vec::new)
                .push(ssid);
        }
    }
    // RSSI
    let mut rssi_table = RSSIS.lock().unwrap();
    // Ajout du dernier RSSI vu
    rssi_table
        .entry(mac_address)
        .and_modify(|rssi_tmp| *rssi_tmp = (rssi).parse::<i32>().unwrap_or(-50))
        .or_insert((rssi).parse::<i32>().unwrap_or(-50));
}

fn fmt_ts(ts: DateTime<Local>) -> Result<String, std::fmt::Error> {
    let mut formatted: String = String::new();
    write!(formatted, "{}", ts.format("%Y-%m-%d -- %H:%M:%S"))?;
    Ok(formatted)
}

#[no_mangle]
#[ffi_function]
pub extern "C" fn get_data_spec<'a>(mac: AsciiPointer<'static>) -> AsciiPointer<'a> {
    let mac_cstr = mac.as_c_str().unwrap();
    let mac_str: String;
    match mac_cstr.to_str() {
        Ok(r) => mac_str = r.to_owned(),
        Err(e) => mac_str = e.to_string(),
    };
    let seen_macs = Arc::clone(&MACS);
    let seen_ssids = Arc::clone(&SSIDS);
    let seen_channels = Arc::clone(&CHANNELS);
    let seen_rssi = Arc::clone(&RSSIS);
    let last_seen = Arc::clone(&LAST_SEEN);
    if let Ok(to_return) = thread::spawn(move || {
        let last_seen = last_seen.lock().unwrap().clone();
        let seen_macs = seen_macs.lock().unwrap().clone();
        let seen_ssids = seen_ssids.lock().unwrap().clone();
        let seen_channels = seen_channels.lock().unwrap().clone();
        let seen_rssi = seen_rssi.lock().unwrap().clone();
        if seen_macs.contains(&mac_str) && MAC_REGEX.is_match(&mac_str) {
            // Format last seen
            let seen_ts: DateTime<Local> = (*last_seen.get(&mac_str).unwrap()).into();
            let seen_ts_str: String;
            match fmt_ts(seen_ts) {
                Ok(r) => seen_ts_str = r,
                Err(_) => seen_ts_str = String::from("0000-00-00 - 00:00:00")
            }
            
            format!(
                "{mac_str} | Last seen : {} | {:?} | {:?} | {:?}\0",
                seen_ts_str,
                seen_channels.get(&mac_str).unwrap(),
                seen_ssids.get(&mac_str),
                seen_rssi.get(&mac_str).unwrap()
            )
        } else {
            format!("\0")
        }
    }).join() {
        println!("{}", to_return);
        AsciiPointer::from_slice_with_nul(to_return.as_bytes()).unwrap()
    } else {
        AsciiPointer::from_slice_with_nul("ERROR\0".as_bytes()).unwrap()
    }
}

#[no_mangle]
#[ffi_function]
pub extern "C" fn get_data_last<'a>() -> AsciiPointer<'a> {
    let seen_macs = Arc::clone(&MACS);
    let seen_ssids = Arc::clone(&SSIDS);
    let seen_channels = Arc::clone(&CHANNELS);
    let seen_rssi = Arc::clone(&RSSIS);
    let last_seen = Arc::clone(&LAST_SEEN);
    let to_return = thread::spawn(move || {
        let seen_macs = seen_macs.lock().unwrap().clone();
        let mac_str = seen_macs.last().unwrap();
        let last_seen = last_seen.lock().unwrap().clone();
        let seen_ssids = seen_ssids.lock().unwrap().clone();
        let seen_channels = seen_channels.lock().unwrap().clone();
        let seen_rssi = seen_rssi.lock().unwrap().clone();
        let seen_ts: DateTime<Local> = (*last_seen.get(mac_str).unwrap()).into();
        format!(
            "{mac_str} | Last seen : {} | {:?} | {:?} | {:?}\0",
            seen_ts.format("%Y-%m-%d -- %H:%M:%S"),
            seen_channels.get(mac_str).unwrap(),
            seen_ssids.get(mac_str),
            seen_rssi.get(mac_str).unwrap()
        )
    })
    .join()
    .unwrap();
    AsciiPointer::from_slice_with_nul(to_return.as_bytes()).unwrap()
}

pub extern "C" fn get_data_all() -> Vec<Data> {
    let data_vec: Vec<Data> = Vec::new();
    let seen_macs = Arc::clone(&MACS);
    let seen_ssids = Arc::clone(&SSIDS);
    let seen_channels = Arc::clone(&CHANNELS);
    let seen_rssi = Arc::clone(&RSSIS);
    let last_seen = Arc::clone(&LAST_SEEN);
    let to_return = thread::spawn(move || {
        let seen_macs = seen_macs.lock().unwrap().clone();
        let mac_str = seen_macs.last().unwrap();
        let last_seen = last_seen.lock().unwrap().clone();
        let seen_ssids = seen_ssids.lock().unwrap().clone();
        let seen_channels = seen_channels.lock().unwrap().clone();
        let seen_rssi = seen_rssi.lock().unwrap().clone();
        let seen_ts: DateTime<Local> = (*last_seen.get(mac_str).unwrap()).into();
        let seen_ts_str: String;
            match fmt_ts(seen_ts) {
                Ok(r) => seen_ts_str = r,
                Err(_) => seen_ts_str = String::from("0000-00-00 - 00:00:00")
        }
        for mac in seen_macs.into_iter() {
            let mac_vec: Data = Data {
                mac: mac,
                rssi: *seen_rssi.get(&mac_str.clone()).clone().unwrap(),
                channels: *seen_channels.get(mac_str).unwrap(),
                ssids: seen_ssids.get(mac_str).unwrap().join(" ")
            };
            data_vec.push(mac_vec);
        }
        data_vec.clone()
        }).join().unwrap();
        data_vec
    }


// Define our FFI interface as `ffi_inventory` containing
// a single function `my_function`. Types are inferred.
pub fn ffi_inventory() -> Inventory {
    InventoryBuilder::new()
        .register(function!(start))
        .register(function!(stop))
        .register(function!(get_data_spec))
        .register(function!(get_data_last))
        .inventory()
}
