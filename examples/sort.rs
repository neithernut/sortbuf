// SPDX-License-Identifier: MIT
//! Simplistic impl of unix `sort` utility
//!
//! This program prints the line-wise sorted concatenation of its inputs. Inputs
//! are supplied as paths via command line arguments. If no inputs are supplied,
//! the program sorts its standart input.


fn main() {
    use std::cmp::Reverse;
    use std::io::BufRead;
    use std::sync::{Arc, Mutex};

    // We need to pre-collect the arguments (minus the progname) since
    // ArgsOs is both `!Send` and `!Sync`.
    let args: Vec<_> = std::env::args_os().skip(1).collect();

    let lines: Arc<Mutex<sortbuf::SortBuf<_>>> = Default::default();

    if args.len() > 0 {
        let paths: Arc<Mutex<_>> = Mutex::new(args.into_iter()).into();

        // We delegate the actual work to multiple worker threads
        let workers: Vec<_> = (0..std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2))
            .map(|_| {
                let paths = paths.clone();
                let lines = lines.clone();
                std::thread::spawn(move || {
                    let mut inserter = sortbuf::Inserter::new(lines);
                    while let Some(path) = paths.lock().unwrap().next() {
                        let lines = std::io::BufReader::new(std::fs::File::open(path).unwrap())
                            .lines()
                            .map(|l| l.unwrap())
                            .map(Reverse);
                        inserter.insert_items(lines).map_err(|(e, _)| e).unwrap()
                    }
                })
            }).collect();

        // Wait for workers to finish...
        workers.into_iter().try_for_each(|t| t.join()).unwrap()
    } else {
        sortbuf::Inserter::new(lines.clone())
            .insert_items(std::io::stdin().lock().lines().map(|l| l.unwrap()).map(Reverse))
            .map_err(|(e, _)| e)
            .unwrap()
    }

    lines.lock().unwrap().take().unreversed().for_each(|l| println!("{}", l));
}

