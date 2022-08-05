// Copyright (c) 2022 - for information on the respective copyright owner see the NOTICE file or the repository https://github.com/hyperledger/indy-node-container.
//
// SPDX-License-Identifier: Apache-2.0

mod thread;
pub(crate) mod worker;

use crate::thread::ThreadedWorker;
use clap::Parser;
use env_logger;
use log::{error, info};
use num_cpus;
use std::thread::JoinHandle;

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
        default_value = "./pool_transactions_genesis"
    )]
    genesis_file: String,

    /// Parallel worker threads
    #[clap(short = 't', long = "threads")]
    threads: Option<u32>,

    /// Time to run for in seconds
    #[clap(short = 'd', long = "duration")]
    duration: Option<u64>,

    /// Reads per write
    #[clap(short = 'r', long = "reads", default_value_t = 0)]
    reads: i8,
}

fn main() {
    env_logger::init();
    let args: Args = Args::parse();

    let seed: String = args.seed;
    let genesis_path: String = args.genesis_file;
    let threads: u32 = args.threads.unwrap_or(num_cpus::get() as u32);

    let mut handles = vec![];

    for n in 0..threads {
        let name = (n + 1).to_string();
        let seed = seed.to_owned();
        let genesis_path = genesis_path.to_owned();

        info!("Spawning worker {}", name);
        let worker = ThreadedWorker::new(seed, genesis_path, name, args.reads);
        match worker {
            Ok(mut worker) => {
                worker.start();
                handles.push(worker);
            }
            Err(err) => {
                error!("Could not create worker: {}", err);
            }
        }
    }

    info!("All workers spawned");
    if args.duration.is_some() {
        let timeout = args.duration.unwrap_or_default();
        info!("Found configured timeout duration: {}", timeout);
        std::thread::sleep(std::time::Duration::from_secs(timeout));
        info!("Timeout expired, shutting down");
        let mut join_handles: Vec<JoinHandle<(u64, u64)>> = vec![];
        for mut worker in handles {
            worker.stop().unwrap();
            let handle = worker.get_handle().unwrap();
            join_handles.push(handle);
        }

        let (mut writes, mut reads) = (0 as u64, 0 as u64);
        for handle in join_handles {
            let (w, r) = handle.join().unwrap();
            writes = writes + w;
            reads = reads + r;
        }
        info!("Writes: {}, Reads: {}", writes, reads);
        info!(
            "Writes/s: {}, Reads/s: {}",
            (writes as f64) / (timeout as f64),
            (reads as f64) / (timeout as f64)
        );
        info!("All workers finished, shutting down");
    } else {
        loop {}
    }
}
