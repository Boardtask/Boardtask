//! Single-writer advisory lock for SQLite. Second process exits with clear error.

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc;
use std::thread;

const MSG: &str = "Another instance of boardtask is already using this database. Stop it first or use a different DATABASE_URL.";

fn lock_path(url: &str) -> Result<Option<PathBuf>, String> {
    if url.contains(":memory:") { return Ok(None); }
    let p = sqlx::sqlite::SqliteConnectOptions::from_str(url).map_err(|e| format!("DATABASE_URL: {}", e))?.get_filename().to_path_buf();
    if p.to_string_lossy().is_empty() || p.to_string_lossy().contains(":memory:") { return Ok(None); }
    let name = p.file_name().map(|n| format!("{}.lock", n.to_string_lossy())).unwrap_or_else(|| "db.lock".into());
    Ok(Some(p.parent().map(|d| d.join(&name)).unwrap_or_else(|| PathBuf::from(name))))
}

pub fn acquire(url: &str) -> Result<Option<SingleWriterGuard>, String> {
    let path = match lock_path(url)? { Some(p) => p, None => return Ok(None) };
    let file = std::fs::OpenOptions::new().create(true).write(true).open(&path).map_err(|e| format!("Lock file {}: {}", path.display(), e))?;
    let (res_tx, res_rx) = mpsc::channel();
    let (exit_tx, exit_rx) = mpsc::channel();
    let join = thread::spawn(move || {
        let mut lock = fd_lock::RwLock::new(file);
        let g = lock.try_write();
        match &g {
            Ok(_) => { let _ = res_tx.send(Ok(())); let _ = exit_rx.recv(); }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => { let _ = res_tx.send(Err(MSG.to_string())); }
            Err(e) => { let _ = res_tx.send(Err(e.to_string())); }
        }
        drop(g);
    });
    match res_rx.recv().map_err(|_| "Lock thread exited without sending".to_string())? {
        Ok(()) => Ok(Some(SingleWriterGuard { exit_tx, join: Some(join) })),
        Err(m) => Err(m),
    }
}

pub struct SingleWriterGuard { exit_tx: mpsc::Sender<()>, join: Option<thread::JoinHandle<()>> }

impl Drop for SingleWriterGuard {
    fn drop(&mut self) { let _ = self.exit_tx.send(()); if let Some(h) = self.join.take() { let _ = h.join(); } }
}
