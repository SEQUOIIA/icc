extern crate icc;

use icc::ping::model::{ConnectivityDown, DurationFormat};
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let mut cd = ConnectivityDown::new();
    cd.start();

    sleep(Duration::from_secs(3));

    cd.end();

    println!("{}", cd.start_end_text());
    println!("{}", cd.duration().as_text());
}