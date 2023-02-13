#[macro_use]
extern crate lazy_static;
extern crate futures;

use std::collections::HashMap;
use std::env;
use std::io;
use std::mem::MaybeUninit;
use std::str;
use std::sync::{Arc, Mutex};
use std::thread;

use bytes::BytesMut;
use futures::stream::StreamExt;
use regex::Regex;
use ringbuf::{Consumer, HeapRb, SharedRb};
use tokio_util::codec::{Decoder, Encoder};
use tokio_serial::SerialPortBuilderExt;

lazy_static! {
    static ref CHANNELS: Mutex<HashMap<String, Vec<u8>>> = {
        let cha = HashMap::new();
        Mutex::new(cha)
    };
    static ref SSIDS: Mutex<HashMap<String, Vec<String>>> = {
        let s = HashMap::new();
        Mutex::new(s)
    };
    static ref RSSIS: Mutex<HashMap<String, Vec<String>>> = {
        let r = HashMap::new();
        Mutex::new(r)
    };
}

#[cfg(windows)]
const DEFAULT_TTY: &str = "COM3";

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

#[tokio::main]
async fn main() -> tokio_serial::Result<()> {
    let mut args = env::args();
    let tty_path = args.nth(1).unwrap_or_else(|| DEFAULT_TTY.into());
    // FIFO queue
    let data_queue = HeapRb::<String>::new(255);
    // Recuperer Producteur et Consommateur
    let (mut data_queue_tx, data_queue_rx) = data_queue.split();
    // Envoi du consommateur dans le thread pour le traitement
    thread::spawn(move || {
        parse_str(data_queue_rx);
    });
    // Ouvrir le port serie
    let port = tokio_serial::new(tty_path, 115_200).open_native_async()?;
    let mut reader = LineCodec.framed(port);
    while let Some(line_result) = reader.next().await {
        let line = line_result.expect("Failed to read line");
        data_queue_tx.push(line).unwrap();
    }
    Ok(())
}

// Silence un warning sur la complexité de type, nécessaire ici
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
        let cleaned_frame: String = frame.replace(&['\u{2}', '\u{3}', '\r', '\n'][..], "");
        // Split au niveau des caracteres de controle
        let splitted_frame: Vec<_> = cleaned_frame.split('\x1F').collect();
        if splitted_frame.len() != 3 || splitted_frame.len() != 4 {
            println!("{:?}", splitted_frame);
        }
        // Check suivant si le channel est a un chiffre ou a deux chiffres
        let channel: u8 = (splitted_frame[0]).parse::<u8>().unwrap_or(0);
        let mac_address: &str = if MAC_REGEX.is_match(splitted_frame[1]) {
            splitted_frame[1]
        } else {
            ""
        };
        let rssi: &str = splitted_frame[2];
        let ssid: &str = if splitted_frame[splitted_frame.len() - 2] != rssi {
            splitted_frame[splitted_frame.len() - 2]
        } else {
            splitted_frame[splitted_frame.len() - 1]
        };
        println!("{channel} {mac_address} {rssi} {ssid}");
        // TODO: Ajouter aux HashMaps
    }
}
