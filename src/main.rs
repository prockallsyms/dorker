/*
 *******************
 * ABUSE THIS TOOL *
 *******************
 */
#![allow(unused)]
extern crate rusqlite;
extern crate structopt;

mod dork;

use self::dork::*;
use rusqlite::{params, Connection, Result, Savepoint};
use std::collections::HashMap;
use std::fs::{metadata, File};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use structopt::StructOpt;

static INSERT: &str =
    "INSERT INTO query_results (engine, dork, extra, link_title, link) VALUES (1?, 2?, 3?, 4?, 5?)";

#[derive(Debug, StructOpt)]
#[structopt(name = "Dorker")]
struct Opts {
    #[structopt(short, long, parse(from_os_str), help = "Sqlite3 DB file to save to")]
    file: Option<PathBuf>,
    #[structopt(short, long, help = "Dork to use | syntax: <search dork>^<content>")]
    dorks: Vec<String>,
    #[structopt(short, long, help = "How many threads to run on")]
    threads: Option<usize>,
    #[structopt(short = "i", long, help = "Extra search term that is not a dork")]
    title: Option<String>,
}

fn main() {
    // Gather the args you supply :)
    let mut matches = Opts::from_args();

    // Organize the dorks
    let mut dorks: HashMap<String, String> = HashMap::new();
    for value in matches.dorks {
        let mut temp: Vec<String> = vec![];
        for token in value.split('^') {
            temp.push(token.to_string());
        }

        dorks.insert(temp[0].clone(), temp[1].clone());
    }

    // Initialize your sqlite db
    let file = matches.file.unwrap_or(Path::new("").to_path_buf());
    let db = init_db(file.as_path());
    let dork_domains = [
        "google.com",
        "www.bing.com",
        "duckduckgo.com",
        "www.ecosia.org",
    ];

    let threads = matches.threads.unwrap_or(1);

    let mut num_threads: usize = 0;

    if threads > dork_domains.len() {
        num_threads = dork_domains.len();
    } else {
        num_threads = threads;
    }

    // Start threads
    let results = DorkResults::new();
    let accumulator = Arc::new(Mutex::new(results));
    let used_dorks = Arc::new(Mutex::new(dorks));
    let extra = Arc::new(Mutex::new(matches.title.unwrap_or("".to_string())));
    let mut handles = vec![];
    for i in 0..num_threads {
        let accumulator = Arc::clone(&accumulator);
        let used_dorks = Arc::clone(&used_dorks);
        let extra = Arc::clone(&extra);
        let handle = thread::spawn(move || {
            let hm1 = used_dorks.lock().unwrap().clone();
            let x1 = extra.lock().unwrap().as_str().to_string();
            println!("scraping on {} with dorks {:?}", dork_domains[i], hm1);
            let scraper = Dork::from(dork_domains[i].to_string(), hm1, x1);
            scraper.get_scrape();
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

pub fn init_db(path: &Path) -> Result<Connection> {
    let conn: Connection = Connection::open_in_memory()?;
    if path.is_file() {
        let meta = metadata(&path).unwrap();
        if meta.permissions().readonly() {
            panic!("File is readonly!")
        }

        let conn = Connection::open(&path);
    } else {
        let mut f = File::create(&path).unwrap_or(File::open("default.db").unwrap());
        let conn = Connection::open(&path);
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS query_result (
            id          INTEGER PRIMARY KEY,
            engine      TEXT NOT NULL,
            dork        TEXT NOT NULL,
            extra       TEXT,
            link_title  TEXT NOT NULL,
            link        TEXT NOT NULL,
        )",
        params![],
    )?;

    Ok(conn)
}
