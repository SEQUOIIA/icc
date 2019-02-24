use super::ping::model::{ConnectivityDown, File};
use std::thread;
use std::io::Write;

pub mod db;

pub fn log_cd(mut cd : ConnectivityDown, log_file : File) {
    thread::spawn(move || {
        let mut file_lock = log_file.lock().unwrap();
        let payload : String = format!("Downtime:\n ({}) {} - ({}) {}\n lasted for: {}\n",
            cd.start_epoch_timestamp(),
            cd.start_text(),
            cd.end_epoch_timestamp(),
            cd.end_text(),
            cd.duration_text());
        file_lock.write_all(payload.as_bytes());

        let dbc = db::Db::new();
        dbc.insert_current_downtime(cd.start_epoch_timestamp(), cd.end_epoch_timestamp());
    });
}