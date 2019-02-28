use super::ping::model::{ConnectivityDown, File};
use std::thread;
use std::io::Write;

pub mod db;
pub mod config;

pub fn log_cd(mut cd : ConnectivityDown, log_file : File, db_filename : String) {
    thread::spawn(move || {
        let mut file_lock_guard = log_file.lock().unwrap();
        let mut file_lock = file_lock_guard.as_mut().unwrap();
        let payload : String = format!("Downtime:\n ({}) {} - ({}) {}\n lasted for: {}\n",
            cd.start_epoch_timestamp(),
            cd.start_text(),
            cd.end_epoch_timestamp(),
            cd.end_text(),
            cd.duration_text());
        file_lock.write_all(payload.as_bytes());

        let dbc = db::Db::new(db_filename.as_str());
        dbc.insert_current_downtime(cd.start_epoch_timestamp(), cd.end_epoch_timestamp());
    });
}