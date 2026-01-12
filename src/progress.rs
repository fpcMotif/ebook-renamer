#![allow(dead_code)]

use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use std::time::Duration;

pub struct ProgressReporter {
    multi: MultiProgress,
    current: Option<ProgressBar>,
    verbose: bool,
}

impl ProgressReporter {
    pub fn new(verbose: bool) -> Self {
        ProgressReporter {
            multi: MultiProgress::new(),
            current: None,
            verbose,
        }
    }

    pub fn start_phase(&mut self, name: &str, total: u64) {
        let pb = self.multi.add(ProgressBar::new(total));
        pb.set_style(
            ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
                .unwrap()
                .progress_chars("##-")
        );
        pb.set_message(name.to_string());
        pb.enable_steady_tick(Duration::from_millis(100));
        self.current = Some(pb);
    }

    pub fn increment(&self, delta: u64) {
        if let Some(ref pb) = self.current {
            pb.inc(delta);
        }
    }

    pub fn finish_phase(&mut self, message: &str) {
        if let Some(pb) = self.current.take() {
            pb.finish_with_message(message.to_string());
        }
    }

    pub fn println(&self, msg: &str) {
        if self.verbose {
            if let Some(ref pb) = self.current {
                pb.println(msg);
            } else {
                println!("{}", msg);
            }
        }
    }
}
