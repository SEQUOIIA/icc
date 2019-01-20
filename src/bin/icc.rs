extern crate pretty_env_logger;
extern crate log;
extern crate icc;

use std::env;
use std::sync::mpsc::{Receiver};
use log::{error, info};

use icc::ping::{PingUtility, PingResult as PingUtilityResult};
use icc::ping::model::{ConnectivityDown};

fn main() {
    setup();

    let (p_utility, results) = PingUtility::new(None).unwrap();

    p_utility.add_ipaddress("8.8.8.8");
    p_utility.add_ipaddress("1.1.1.1");

    p_utility.start_pinging();

    let mut cd_col : Vec<ConnectivityDown> = Vec::new();
    let mut cd : ConnectivityDown = ConnectivityDown::new();

    loop {
        match results.recv() {
            Ok(res) => {
                match res {
                    PingUtilityResult::Response{addr, rtt, sequence, identifier} => {
                        info!("Receive from Address {} in {:?}. seq = {}, identifier = {}", addr, rtt, sequence, identifier);

                        if cd.is_started() {
                            cd.end();
                        }
                    },

                    PingUtilityResult::Timeout {addr} => {
                        error!("Idle Address {}.", addr);

                        if !cd.is_started() {
                            cd.start(); // Start tracking of downtime
                        }
                    }
                }
            },
            Err(_) => panic!("Something went wrong during result loop")
        }

        if cd.is_ready() {
            cd_col.push(cd);
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