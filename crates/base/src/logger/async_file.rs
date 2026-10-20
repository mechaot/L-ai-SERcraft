//! Port of `Logger/AsyncLogFile.cs`: queue + writer thread.

use std::collections::VecDeque;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

/// Max queued entries; extra ones dropped like C#.
const MAX_QUEUE: usize = 10000;

struct Shared {
    q: Mutex<(VecDeque<String>, bool)>, // (queue, exit)
    cv: Condvar,
}

/// Async append-only log file.
pub struct AsyncLogFile {
    shared: Arc<Shared>,
    th: Mutex<Option<JoinHandle<()>>>,
    filename: PathBuf,
}

impl AsyncLogFile {
    /// Rotate and start the writer thread.
    ///
    /// `filename`: log path. `maxline`: 0 = delete old file, `u32::MAX` = keep all, else keep last N lines.
    pub fn new(filename: PathBuf, maxline: u32) -> Self {
        rotate_log(&filename, maxline);
        let shared = Arc::new(Shared { q: Mutex::new((VecDeque::new(), false)), cv: Condvar::new() });
        let (s2, f2) = (shared.clone(), filename.clone());
        let th = std::thread::Builder::new().name(format!("AsyncLog {}", filename.display())).spawn(move || do_the_work(s2, f2)).ok();
        Self { shared, th: Mutex::new(th), filename }
    }

    /// Enqueue text (dropped when queue full).
    ///
    /// `text`: raw text, already with line ends.
    pub fn log(&self, text: &str) {
        let mut g = self.shared.q.lock().unwrap_or_else(|e| e.into_inner());
        if g.0.len() < MAX_QUEUE {
            g.0.push_back(text.to_string());
        }
        drop(g);
        self.shared.cv.notify_one();
    }

    /// Ask the writer to drain the queue and exit; waits for it.
    pub fn stop(&self) {
        self.shared.q.lock().unwrap_or_else(|e| e.into_inner()).1 = true;
        self.shared.cv.notify_one();
        if let Some(h) = self.th.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = h.join();
        }
    }

    /// True when the file exists.
    pub fn exist_log(&self) -> bool {
        self.filename.exists()
    }

    /// File path.
    pub fn path(&self) -> &Path {
        &self.filename
    }

    /// Open file with the system viewer.
    pub fn show_log(&self) {
        crate::tools::utils::open_path(&self.filename);
    }
}

/// Writer thread body: wait, drain queue to file, repeat until exit.
///
/// `sh`: shared queue. `filename`: target.
fn do_the_work(sh: Arc<Shared>, filename: PathBuf) {
    loop {
        let (items, exit) = {
            let mut g = sh.q.lock().unwrap_or_else(|e| e.into_inner());
            while g.0.is_empty() && !g.1 {
                g = sh.cv.wait(g).unwrap_or_else(|e| e.into_inner());
            }
            (std::mem::take(&mut g.0), g.1)
        };
        if !items.is_empty() {
            // open append per batch like C# StreamWriter(append)
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&filename) {
                for s in &items {
                    let _ = f.write_all(s.as_bytes());
                }
            } else {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        if exit {
            break;
        }
    }
}

/// Trim file to the last `maxline` lines (C# `RotateLog`).
///
/// `filename`: log path. `maxline`: 0 = delete, `u32::MAX` = untouched.
fn rotate_log(filename: &Path, maxline: u32) {
    if !filename.exists() {
        return;
    }
    if maxline == 0 {
        let _ = std::fs::remove_file(filename);
    } else if maxline != u32::MAX
        && let Ok(bytes) = std::fs::read(filename)
    {
        let text = String::from_utf8_lossy(&bytes);
        let lines: Vec<&str> = text.split_inclusive('\n').collect();
        if lines.len() > maxline as usize {
            let keep = lines[lines.len() - maxline as usize..].concat();
            let _ = std::fs::write(filename, keep);
        }
    }
}
