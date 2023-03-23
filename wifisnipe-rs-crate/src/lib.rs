//! Traite les données arrivant du programme `esp-sniffer` pour NodeMCU
//!
//! ## Schéma
//! Utilise les structures suivantes en entrée :
//!
//! Si le SSID est vide
//! - `\u{2}canal\u{31}mac\u{31}rssi\{31}\u{3}`
//!
//! Si le SSID est présent
//! - `\u{2}canal\u{31}mac\u{31}rssi\{31}ssid\{31}\u{3}`
//!
//! ## Compilation :
//!
//! - Installez [git](https://git-scm.com/download/win) et [gh](https://github.com/cli/cli)
//! - Installez [Visual Studio](https://visualstudio.microsoft.com/) en sélectionnant `Développement Desktop en C++` dans `Charges de Travail` et `Anglais` dans `Modules Linguistiques`
//! - Installez [Rustup](https://rustup.rs/), l'installateur de Rust
//! - Ouvrez une fenêtre PowerShell en tant qu'Administrateur et exécuter les commandes suivantes :
//!
//! ```ps1
//! # Se connecter a votre compte GitHub
//! gh auth login
//! # Installez Rust Stable
//! rustup toolchain install stable
//! # Cloner la repo
//! gh repo clone multipower-fr/esp-sniffer
//! # Allez dans le dossier de la *crate* (librairie)
//! cd esp-sniffer\wifisnipe-rs-crate
//! # Compiler la libarie (enlever le --release pour la version non-optimisée de développement)
//! cargo build --release
//! ```
//! Vous trouverez le `.dll` dans `target\release` (ou `target\debug` en cas de compilation en développement)
//!
//! ## Interface
//!
//! La librarie [`interoptopus`] permets de générer des fichiers d'interface
//!
//! Le fichier `tests\bindings.rs` génère le fichier pour Python.
//!
//! Il est possible de les générer automatiquement en suivant les documentations suivantes
//! 
//! - `C#` ([`interoptopus_backend_csharp`])
//! - `C` ([`interoptopus_backend_c`])
//! - `Python` ([`interoptopus_backend_cpython`])
//!
//! Une fois créé, les interfaces peuvent être générées par la commande `cargo test`
//! 
//! Il est également possible de les créer manuellement
//!
//! ## Utilisation
//!
//! Vous pouvez utiliser le pseudo-code suivant comme base :
//!
//! ```lua
//! load_dll()
//! # Démarrage de l'enregistrement
//! start(<Numéro du port COM (entier sans COM)>)
//! # Dernier appareil enregistré
//! get_data_last()
//! # Toute les données enregistrées
//! get_data_all()
//! # Arrêter l'enregistrement
//! stop()
//! ```

#![deny(rustdoc::broken_intra_doc_links)]
#![deny(missing_docs)]
#[macro_use]
extern crate lazy_static;
extern crate futures;

use chrono::{DateTime, Utc};
use interoptopus::patterns::string::*;
use interoptopus::{ffi_function, function, Inventory, InventoryBuilder};
use std::collections::HashMap;
use std::io;
use std::mem::MaybeUninit;
use std::str;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::SystemTime;

use bytes::BytesMut;
use futures::stream::StreamExt;
use regex::Regex;
use ringbuf::{Consumer, HeapRb, SharedRb};
use serde::{Deserialize, Serialize};
use tokio_serial::SerialPortBuilderExt;
use tokio_util::codec::{Decoder, Encoder};

// Toutes les instantations globales
lazy_static! {
    /// Tableau des adresses MAC
    static ref MACS: Arc<Mutex<Vec<String>>> = {
        let macs: Vec<String> = Vec::new();
        Arc::new(Mutex::new(macs))
    };
    /// HashMap avec les canaux ou certaines MAC sont visibles
    static ref CHANNELS: Arc<Mutex<HashMap<String, Vec<u32>>>> = {
        let cha = HashMap::new();
        Arc::new(Mutex::new(cha))
    };
    /// HashMap avec les SSIDs récupérés
    static ref SSIDS: Arc<Mutex<HashMap<String, Vec<String>>>> = {
        let s = HashMap::new();
        Arc::new(Mutex::new(s))
    };
    /// HashMap avec le dernier RSSI
    static ref RSSIS: Arc<Mutex<HashMap<String, i32>>> = {
        let r = HashMap::new();
        Arc::new(Mutex::new(r))
    };
    /// HashMap avec les timestamp ou les addresses MAC sont vues en dernier
    static ref LAST_SEEN: Arc<Mutex<HashMap<String, SystemTime>>> = {
        let ts = HashMap::new();
        Arc::new(Mutex::new(ts))
    };
    /// Regex pour la vérification syntaxique de l'addresse MAC
    static ref MAC_REGEX: Regex =
        Regex::new(r"^([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})$").unwrap();
    static ref STARTED: AtomicBool = AtomicBool::new(false);
    // Signal de stop
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
/// Structure de données pour la sérialisation en JSON
///
/// Champs:
/// | Champ      | Type            | Description                           |
/// |------------|-----------------|---------------------------------------|
/// | `mac`      | `String`        | Adresse MAC                           |
/// | `ts`       | `int`           | UNIX Timestamp (UTC)                  |
/// | `rssi`     | `int`           | RSSI                                  |
/// | `channels` | `Array<int>`    | Canaux où le périphérique a été vu    |
/// | `ssid`     | `Array<String>` | SSIDs broadcastés par le périphérique |
///
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data {
    // Addresse MAC
    mac: String,
    // UNIX Timestamp (UTC)
    ts: i64,
    // RSSI
    rssi: i32,
    // Tableau de canaux
    channels: Vec<u32>,
    // Tableau des SSIDs
    ssids: Vec<String>,
}

#[tokio::main]
/// Bootstrap le traitement
async fn serial_port(port_name: String) -> tokio_serial::Result<()> {
    let port = tokio_serial::new(port_name, 115_200).open_native_async()?;
    STARTED.store(true, Ordering::SeqCst);
    let mut reader = LineCodec.framed(port);
    // FIFO queue
    let data_queue = HeapRb::<String>::new(255);
    // Recuperer Producteur et Consommateur
    let (mut data_queue_tx, data_queue_rx) = data_queue.split();
    // Envoi du consommateur dans le thread et dans la fonction parse_str pour le traitement
    thread::spawn(move || {
        parse_str(data_queue_rx);
    });
    while let Some(line_result) = reader.next().await {
        let line = line_result.expect("Failed to read line");
        // Réponds au signal de stop
        if STOP.load(Ordering::SeqCst) {
            STARTED.store(false, Ordering::SeqCst);
            STOP.store(false, Ordering::SeqCst);
            break;
        }
        // Si la ligne n'est pas mauvaise, la push sur le FIFO
        data_queue_tx.push(line).unwrap();
    }
    Ok(())
}

/// Fonction publique pour démarrer l'enregistrement
///
/// Paramètres :
///
/// | Nom du paramètre | Usage |
/// | ---------------- | ----- |
/// | `tty_no` | Numéro du port COM (ex. `3` => `COM3`) |
///
/// Retourne
///   - `false` : Si le système était stoppé
///   - `true` Si le système était déjà démarré
#[no_mangle]
#[ffi_function]
#[cfg(windows)]
pub extern "C" fn start(tty_no: u32) -> bool {
    if !STARTED.load(Ordering::SeqCst) {
        let mut port_name: String = "COM".to_owned();
        // Signal d'arret de l'enregistrement
        STOP.store(false, Ordering::SeqCst);
        port_name.push_str(tty_no.to_string().as_str());
        thread::spawn(move || {
            serial_port(port_name).unwrap();
        });
        false
    } else {
        true
    }
}

/// Fonction publique pour stopper l'enregistrement
///
/// Retourne :
///   - `false` Si le système était démarré
///   - `true` Si le système était déjà stoppé
#[no_mangle]
#[ffi_function]
pub extern "C" fn stop() -> bool {
    if STARTED.load(Ordering::SeqCst) {
        // Demande l'arret
        STOP.store(true, Ordering::SeqCst);
        false
    } else {
        // Signaler que le système est déjà arrêté
        true
    }
}

#[allow(clippy::type_complexity)]
/// Décompose et récupère les données
fn parse_str(mut data_queue_rx: Consumer<String, Arc<SharedRb<String, Vec<MaybeUninit<String>>>>>) {
    loop {
        while data_queue_rx.is_empty() {
            // Ne rien faire si la queue est vide
        }
        // Recupere un element de la FIFO
        let frame: String = data_queue_rx.pop().unwrap();
        // Clean la trame en enlevant tout les caractères spéciaux qui nous interesse pas
        let cleaned_frame: String = frame.replace(&['\u{2}', '\u{3}', '\r', '\n'][..], "");
        // Split au niveau des caracteres de controle
        let splitted_frame: Vec<_> = cleaned_frame.split('\x1F').collect();
        // Decode le channel
        let channel: u32 = (splitted_frame[0]).parse::<u32>().unwrap_or(0);
        // Verifie la syntaxe de l'addresse
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

/// Enregistre les données récupérées
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
        .and_modify(|rssi_tmp| *rssi_tmp = (rssi).parse::<i32>().unwrap_or(0))
        .or_insert((rssi).parse::<i32>().unwrap_or(0));
}

#[no_mangle]
#[ffi_function]
/// Bundle des données en mémoire récoletées pour le dernier appareil pour la génération d'un string JSON
///
/// Retourne un `const char *`, encodé en UTF-8, et terminé en NULL (`\0`)
///
/// Le timestamp UNIX produit est en UTC généré par [`chrono::DateTime<Utc>`]
///
/// En cas d'erreur les valeurs suivantes vont être appliquées dans le JSON
///
/// | Champ | Type | Valeur en cas d'erreur |
/// | ----- | ---- | ---------------------- |
/// | rssi  | `i32`  | `0` |
/// | channels | `Vec<u32>` | `[0]` |
/// | ssids | `Vec<String>` | `[""]` |
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
        let seen_ts: DateTime<Utc> = (*last_seen.get(mac_str).unwrap()).into();
        let unix_utc_ts: i64 = seen_ts.timestamp();
        let mac_vec: Data = Data {
            mac: mac_str.to_owned(),
            ts: unix_utc_ts,
            rssi: seen_rssi.get(mac_str).unwrap_or(&0).to_owned(),
            channels: seen_channels.get(mac_str).unwrap_or(&vec![0]).to_owned(),
            ssids: seen_ssids
                .get(mac_str)
                .unwrap_or(&vec![String::from("")])
                .to_owned(),
        };
        json_serialize(mac_vec)
    })
    .join()
    .unwrap();
    let ret = to_return.unwrap_or(String::from(""));
    let mut nulled_safe_ret = ret.replace('\0', "");
    nulled_safe_ret.push('\0');
    AsciiPointer::from_slice_with_nul(nulled_safe_ret.as_bytes())
        .unwrap_or(AsciiPointer::from_slice_with_nul(String::from("\0").as_bytes()).unwrap())
}

#[cfg(feature = "json")]
#[no_mangle]
#[ffi_function]
/// Bundle de toutes les données en mémoire pour la génération d'un fichier JSON
///
/// Retourne un `const char *`, encodé en UTF-8, et terminé en NULL (`\0`)
///
/// Le timestamp UNIX produit est en UTC généré par [`chrono::DateTime<Utc>`]
///
/// En cas d'erreur les valeurs suivantes vont être appliquées dans le JSON
///
/// | Champ | Type | Valeur en cas d'erreur |
/// | ----- | ---- | ---------------------- |
/// | rssi  | `i32` | `0` |
/// | channels | `Vec<u32>` | `[0]` |
/// | ssids | `Vec<String>` | `[""]` |
pub extern "C" fn get_data_all<'a>() -> AsciiPointer<'a> {
    let mut data_vec: Vec<Data> = Vec::new();
    let seen_macs = Arc::clone(&MACS);
    let seen_ssids = Arc::clone(&SSIDS);
    let seen_channels = Arc::clone(&CHANNELS);
    let seen_rssi = Arc::clone(&RSSIS);
    let last_seen = Arc::clone(&LAST_SEEN);
    let to_return = thread::spawn(move || {
        let seen_macs = seen_macs.lock().unwrap().clone();
        let last_seen = last_seen.lock().unwrap().clone();
        let seen_ssids = seen_ssids.lock().unwrap().clone();
        let seen_channels = seen_channels.lock().unwrap().clone();
        let seen_rssi = seen_rssi.lock().unwrap().clone();
        for mac in seen_macs.into_iter() {
            let seen_ts: DateTime<Utc> = (*last_seen.get(&mac).unwrap()).into();
            let unix_utc_ts: i64 = seen_ts.timestamp();
            let mac_vec: Data = Data {
                mac: mac.to_owned(),
                ts: unix_utc_ts,
                rssi: seen_rssi.get(&mac).unwrap_or(&0).to_owned(),
                channels: seen_channels.get(&mac).unwrap_or(&vec![0]).to_owned(),
                ssids: seen_ssids
                    .get(&mac)
                    .unwrap_or(&vec![String::from("")])
                    .to_owned(),
            };
            data_vec.push(mac_vec);
        }
        json_serialize(data_vec)
    })
    .join()
    .unwrap();
    let mut ret = to_return.unwrap_or(String::from(""));
    ret.push('\0');
    AsciiPointer::from_slice_with_nul(ret.as_bytes())
        .unwrap_or(AsciiPointer::from_slice_with_nul(String::from("\0").as_bytes()).unwrap())
}

#[cfg(feature = "json")]
/// Convertis en JSON les structures envoyées depuis [`get_data_last()`] et [`get_data_all()`]
fn json_serialize(data_vec: impl Serialize) -> Result<String, serde_json::Error> {
    serde_json::to_string(&data_vec)
}

/// Inventaire pour la génèration des fichiers d'accès
///
/// Utilise [`interoptopus::InventoryBuilder`]
pub fn ffi_inventory() -> Inventory {
    InventoryBuilder::new()
        .register(function!(start))
        .register(function!(stop))
        .register(function!(get_data_all))
        .register(function!(get_data_last))
        .inventory()
}
