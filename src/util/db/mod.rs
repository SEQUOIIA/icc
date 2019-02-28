extern crate rusqlite;

use rusqlite::{Connection, Result, NO_PARAMS, Statement, OpenFlags};

pub struct Db {
    pub conn : Connection
}

impl Db {
    pub fn new(filename : &str) -> Self {
        //let conn = Connection::open("data").unwrap();
        let conn = Connection::open_with_flags(filename, OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_SHARED_CACHE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_URI).unwrap();

        conn.execute("create table if not exists current_downtime (\
                            id integer primary key,\
                            start integer,\
                            end integer\
                       )",
                     NO_PARAMS).unwrap();

        Self {conn: conn}
    }

    pub fn insert_current_downtime(&self, start : i64, end : i64) {
        let mut insert_current_downtime : Statement = self.conn.prepare("INSERT INTO current_downtime (start, end) values (?1, ?2)").unwrap();

        insert_current_downtime.execute(&[&start, &end]);
    }
}