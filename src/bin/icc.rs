extern crate pretty_env_logger;
extern crate log;
extern crate icc;
extern crate ctrlc;

use std::env;
use std::fs::{File, OpenOptions};
use std::sync::mpsc::{Receiver};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use log::{error, info, debug};

use icc::ping::{PingUtility, PingResult as PingUtilityResult};
use icc::ping::model::{ConnectivityDown};
use icc::util::log_cd;
use icc::util::db::Db;
use icc::util::config::{config, Config};

fn handle_exit(mut stop_sig : Arc<AtomicBool>) {
    ctrlc::set_handler(move || {
        info!("Stopping ICC");
        stop_sig.swap(true, Ordering::Relaxed);
    }).expect("Unable to set SIGINT handler");
}

fn main() {
    let config : Config = config();

    setup();

    let stop_bool = Arc::new(AtomicBool::new(false));
    handle_exit(stop_bool.clone());

    let (p_utility, results) = PingUtility::new(Some(config.max_ping_timeout.as_ref().unwrap().clone())).unwrap();

    for ip in config.addresses_to_monitor.as_ref().unwrap() {
        p_utility.add_ipaddress(ip);
    }

    p_utility.start_pinging();

    let db_client = Db::new(config.db.as_ref().unwrap());
    let mut log_file : Arc<Mutex<Option<File>>>;
    let mut use_clear_text_log : bool = false;
    if let Some(filename) = config.clear_text_log {
        use_clear_text_log = true;
        log_file = Arc::new(Mutex::new(Some(OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .append(true)
            .open(filename).unwrap())));
    } else {
        log_file = Arc::new(Mutex::new(None))
    }

    let mut cd_col : Vec<ConnectivityDown> = Vec::new();
    let mut cd : ConnectivityDown = ConnectivityDown::new();
    let mut no_response_counter = 0;
    let no_response_counter_limit = config.max_timeouts.as_ref().unwrap().clone();

    loop {
        match results.recv() {
            Ok(res) => {
                match res {
                    PingUtilityResult::Response{addr, rtt, sequence, identifier} => {
                        info!("Receive from Address {} in {:?}. seq = {}, identifier = {}", addr, rtt, sequence, identifier);

                        if cd.is_started() {
                            if no_response_counter >= no_response_counter_limit {
                                cd.end();
                            } else {
                                cd = ConnectivityDown::new();
                            }
                        }
                        if no_response_counter != 0 {
                            no_response_counter = 0;
                            debug!("no_response_counter reset to 0");
                        }
                    },

                    PingUtilityResult::Timeout {addr} => {
                        error!("Idle Address {}.", addr);
                        if no_response_counter < no_response_counter_limit {
                            no_response_counter = no_response_counter + 1;
                            debug!("no_response_counter increased with 1, currently at {}", no_response_counter);
                        }
                        if !cd.is_started() {
                            cd.start(); // Start tracking of downtime
                        }
                    },
                    _ => {}
                }
            },
            Err(_) => panic!("Something went wrong during result loop")
        }

        if cd.is_ready() {
            cd_col.push(cd);
            if use_clear_text_log {
                icc::util::THREADS_ACTIVE_GRACEFUL.fetch_add(1, Ordering::SeqCst);
                log_cd(cd.clone(), log_file.clone(), config.db.as_ref().unwrap().to_owned());
            }
            cd = ConnectivityDown::new();
        }

        if stop_bool.load(Ordering::Relaxed) {
            break;
        }
    }

    while icc::util::THREADS_ACTIVE_GRACEFUL.load(Ordering::SeqCst) != 0 {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}


#[cfg(debug_assertions)]
fn setup() {
    env::set_var("RUST_LOG", "trace");
    pretty_env_logger::init();
}

#[cfg(not(debug_assertions))]
fn setup() {

}