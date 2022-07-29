// Copyright (c) 2022 - for information on the respective copyright owner see the NOTICE file or the repository https://github.com/hyperledger/indy-node-container.
//
// SPDX-License-Identifier: Apache-2.0

mod worker;

use crate::worker::worker::spawn_worker;
use clap::Parser;
use env_logger;
use futures::future::join_all;
use log::info;
use std::time::Duration;

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
    #[clap(short = 't', long = "threads", default_value = "10")]
    threads: u32,

    /// Time to run for in seconds
    #[clap(short = 'd', long = "duration", default_value = "15")]
    duration: u64,
}
#[tokio::main(flavor = "multi_thread", worker_threads = 15)]
async fn main() {
    env_logger::init();
    let args: Args = Args::parse();

    let seed: String = args.seed;
    let genesis_path: String = args.genesis_file;
    let threads: u32 = args.threads;
    let duration = Duration::from_secs(args.duration);

    let mut handles = vec![];

    for n in 0..threads {
        let name = (n + 1).to_string();
        let seed = seed.to_owned();
        let genesis_path = genesis_path.to_owned();
        let duration = duration.to_owned();

        info!("Creating worker {}", name);

        //let join_handle = rt.spawn(spawn_worker(seed, genesis_path, name));
        let join_handle = tokio::spawn(tokio::time::timeout(
            duration,
            spawn_worker(seed, genesis_path, name),
        ));
        handles.push(join_handle);
    }
    info!("All workers spawned, waiting for defined timeout");
    let _ = join_all(handles);
}
