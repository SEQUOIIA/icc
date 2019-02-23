extern crate pretty_env_logger;
extern crate log;
extern crate icc;

use std::env;
use std::fs::{File, OpenOptions};
use std::sync::mpsc::{Receiver};
use std::sync::{Arc, Mutex};
use log::{error, info, debug};

use icc::ping::{PingUtility, PingResult as PingUtilityResult};
use icc::ping::model::{ConnectivityDown};
use icc::util::log_cd;

fn main() {
    setup();

    let (p_utility, results) = PingUtility::new(None).unwrap();

    p_utility.add_ipaddress("8.8.8.8");
    p_utility.add_ipaddress("1.1.1.1");

    p_utility.start_pinging();

    let log_file = Arc::new(Mutex::new(OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("icc-log").unwrap()));

    let mut cd_col : Vec<ConnectivityDown> = Vec::new();
    let mut cd : ConnectivityDown = ConnectivityDown::new();
    let mut no_response_counter = 0;
    let no_response_counter_limit = 3;

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
                        no_response_counter = 0;
                        debug!("no_response_counter reset to 0");
                    },

                    PingUtilityResult::Timeout {addr} => {
                        error!("Idle Address {}.", addr);
                        no_response_counter = no_response_counter + 1;
                        debug!("no_response_counter increased with 1, currently at {}", no_response_counter);
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
            log_cd(cd.clone(), log_file.clone());
            cd = ConnectivityDown::new();
        }
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