#[macro_use]
extern crate lazy_static;
extern crate futures;

use chrono::{DateTime, Local};
use std::collections::HashMap;
use std::io;
use std::mem::MaybeUninit;
use std::str;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::SystemTime;
use std::ffi::CStr;
use std::ffi::c_char;
use ::safer_ffi::prelude::*;

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
    // Timestamp du dernier print (ici statique)
    static ref LAST_PRINT: SystemTime = SystemTime::now();
    // Nombre de print effectués
    static ref PRINT_COUNT: AtomicU64 = AtomicU64::new(1);
}

#[derive_ReprC]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ReturnedData {
    mac: Vec<usize>,
    channels: Vec<u32>,
    rssi: i32,


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
        // Si la ligne n'est pas mauvaise, la push sur le FIFO
        data_queue_tx.push(line).unwrap();
    }
    Ok(())
}


#[tokio::main]
#[ffi_export]
pub async extern "C" fn start(tty_no: usize) {
    let mut port_name: String = "COM".to_owned();
    port_name.push_str(tty_no.to_string().as_str());
    thread::spawn(move || {
        serial_port(port_name);
    });
}

#[allow(clippy::type_complexity)]
fn parse_str(mut data_queue_rx: Consumer<String, Arc<SharedRb<String, Vec<MaybeUninit<String>>>>>) {
    loop {
        while data_queue_rx.is_empty() {
            // Ne rien faire si la queue est vide
        }
        lazy_static! {
            static ref MAC_REGEX: Regex =
                Regex::new(r"^([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})$").unwrap();
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

fn store(channel: u8, mac_address: String, rssi: String, ssid: String) {
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
        .and_modify(|rssi_tmp| *rssi_tmp = (rssi).parse::<i16>().unwrap_or(-50))
        .or_insert((rssi).parse::<i32>().unwrap_or(-50));
}

pub extern fn print(mac: *const c_char) {
    let c_str: &CStr = unsafe { CStr::from_ptr(mac) };
    let str_slice: String = String::from_utf8_lossy(c_str.to_bytes()).to_string();
    let seen_macs = Arc::clone(&MACS);
    let seen_ssids = Arc::clone(&SSIDS);
    let seen_channels = Arc::clone(&CHANNELS);
    let seen_rssi = Arc::clone(&RSSIS);
    let last_seen = Arc::clone(&LAST_SEEN);
    thread::spawn(move || {
        let seen_macs = seen_macs.lock().unwrap().clone();
        let seen_ssids = seen_ssids.lock().unwrap().clone();
        let seen_channels = seen_channels.lock().unwrap().clone();
        let seen_rssi = seen_rssi.lock().unwrap().clone();
        let last_seen = last_seen.lock().unwrap().clone();
        println!(
            "---------- {} ----------",
            Local::now().format("%Y-%m-%d][%H:%M:%S")
        );
        for mac in seen_macs.into_iter() {
            // Convertis le timestamp dans un format qui permets l'affichage
            let seen_ts: DateTime<Local> = (*last_seen.get(&mac.clone()).unwrap()).into();
            let diff = Local::now() - seen_ts;
            if diff.num_minutes() >= 30 {
                continue;
            }
            println!(
                "{mac} | Last seen : {} | {:?} | {:?} | {:?}",
                seen_ts.format("%Y-%m-%d -- %H:%M:%S"),
                seen_channels.get(&mac.clone()).unwrap(),
                seen_ssids.get(&mac.clone()),
                seen_rssi.get(&mac.clone()).unwrap()
            )
        }
    });
}
