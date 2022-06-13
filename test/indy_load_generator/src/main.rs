mod worker;

use clap::Parser;
use crate::worker::run;
use std::thread;
use std::thread::JoinHandle;
use env_logger;
use std::time::Duration;
use log::{info, debug, error};


#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Seed to sign transactions with
    #[clap(
        short = 's',
        long = "seed",
        default_value = "000000000000000000000000Trustee1"
    )]
    seed: String,

    /// Pool transaction genesis filename
    #[clap(
        short = 'g',
        long = "genesis",
        default_value = "/pool_transactions_genesis"
    )]
    genesis_file: String,

    /// Parallel worker threads
    #[clap(
    short = 't',
    long = "threads",
    default_value = "10"
    )]
    threads: i32,

    /// Time to run for in seconds
    #[clap(
    short = 'd',
    long = "duration",
    default_value = "15"
    )]
    duration: u64,
}


fn main() {
    env_logger::init();
    let args = Args::parse();

    let seed: String = args.seed;
    let genesis_path: String = args.genesis_file;
    let threads: i32 = args.threads;
    let duration = Duration::from_secs(args.duration);

    let mut handles= Vec::new();
    for n in 1..threads+1 {
        let seed = seed.to_owned();
        let genesis_path= genesis_path.to_owned();
        handles.push(thread::spawn(move || {
            debug!("Spawning Thread {}", n);
            let res = run(seed, genesis_path, duration, n);
            match res {
                Ok(_) => {
                    info!("Worker {} finished successfully", n)
                },
                Err(e) => {
                    error!("Worker {} ran into an error: {}", n, e)
                }
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
}
