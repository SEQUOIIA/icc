extern crate chrono;
extern crate time;

use std::time::{Duration as DurationStd, Instant};
use std::clone::Clone;
use chrono::prelude::{Local, DateTime, TimeZone};
use time::Duration;
use std::sync::{Arc, Mutex};
use std::fs::File as fsFile;

pub type File = Arc<Mutex<Option<fsFile>>>;

#[derive(Clone, Copy)]
pub struct ConnectivityDown {
    start : Option<i64>,
    end : Option<i64>,
    is_started : bool,
}

impl ConnectivityDown {
    pub fn new() -> Self {
        Self {start: None, end: None, is_started: false}
    }

    pub fn is_ready(&self) -> bool {
        if self.start.is_none() {
            return false;
        }

        if self.end.is_none() {
            return false;
        }

        return true;
    }

    pub fn is_started(&self) -> bool {
        self.is_started
    }

    pub fn start(&mut self) {
        if self.start.is_none() {
            self.start = Some(Local::now().timestamp());
            self.is_started = true;
        }
    }

    pub fn end(&mut self) {
        if self.end.is_none() {
            self.end = Some(Local::now().timestamp());
        }
    }

    pub fn duration(&self) -> Duration {
        let start = Local.timestamp(self.start.unwrap(), 0);
        let end = Local.timestamp(self.end.unwrap(), 0);
        end.signed_duration_since(start)
    }

    pub fn start_end_text(&self) -> String {
        let start = Local.timestamp(self.start.unwrap(), 0);
        let end = Local.timestamp(self.end.unwrap(), 0);
        format!("{} - {}", start.to_rfc2822(), end.to_rfc2822())
    }

    pub fn start_text(&self) -> String {
        let start = Local.timestamp(self.start.unwrap(), 0);
        start.to_rfc2822()
    }

    pub fn start_epoch_timestamp(&self) -> i64 {
        self.start.unwrap()
    }

    pub fn end_text(&self) -> String {
        let end = Local.timestamp(self.end.unwrap(), 0);
        end.to_rfc2822()
    }

    pub fn end_epoch_timestamp(&self) -> i64 {
        self.end.unwrap()
    }

    pub fn duration_text(&self) -> String {
        let v = self.duration();
        let hours = v.num_hours();
        let minutes = v.num_minutes() - (v.num_hours() * 60);
        let seconds = v.num_seconds() - ((v.num_hours() * 60 * 60) + (minutes * 60));
        format!("{} hours, {} minutes, {} seconds", hours, minutes, seconds).to_owned()
    }
}

impl DurationFormat for Duration {
    fn as_text(&self) -> String {
        let hours = self.num_hours();
        let minutes = self.num_minutes() - (self.num_hours() * 60);
        let seconds = self.num_seconds() - ((self.num_hours() * 60 * 60) + (minutes * 60));
        format!("{} hours, {} minutes, {} seconds", hours, minutes, seconds).to_owned()
    }
}

pub trait DurationFormat {
    fn as_text(&self) -> String;
}