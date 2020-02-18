use crossbeam::channel;
use std::env;
use std::io::{self, Write};
use walkdir::{DirEntry,  WalkDir};
use rayon::prelude::*;
use fuzzyrs::{Pattern, MatchOptions};

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

fn read_from_current_dir() {
    let term = env::args().nth(1).unwrap();
    
    let options = MatchOptions::default();
    let pattern = Pattern::new(&term, options);
    
    let (sender, receiver) = channel::unbounded::<String>();

    let current_dir = env::current_dir().unwrap();
    let path_len = current_dir.to_str().unwrap().len() + 1;
    let input: Vec<DirEntry> = WalkDir::new(current_dir)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();
    
    let thread = std::thread::spawn(move || {
        let stdout = io::stdout();
        let mut writer = stdout.lock();
        for line in receiver.into_iter() {
            writeln!(writer, "{}", line).unwrap();
        }
    });

    input.as_parallel_slice()
        .par_chunks(32)
        .for_each_with(sender, |sender, chunk| {
            for entry in chunk {
                let full_path = entry.path().to_str().unwrap();
                let rel_path = &full_path[path_len..full_path.len()];
                if pattern.matches(rel_path.as_bytes()).is_some() {
                    sender.send(rel_path.to_string()).unwrap()
                }
            }
        });

    thread.join().unwrap();
}

fn main() {
    read_from_current_dir()
}
