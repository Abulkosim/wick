//! Small files that one process replaces while another reads them: `status.json` and the
//! agent holds. On Windows a rename or delete fails while any reader has the file open
//! without delete sharing, so readers allow it and writers retry for a moment.

use std::{fs, io, path::Path, thread::sleep, time::Duration};

const TRIES: u32 = 5;
const PAUSE: Duration = Duration::from_millis(10);

/// Opens for reading without blocking a writer's rename or delete.
pub fn open(path: &Path) -> io::Result<fs::File> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const SHARE_ALL: u32 = 0x1 | 0x2 | 0x4;
        fs::OpenOptions::new()
            .read(true)
            .share_mode(SHARE_ALL)
            .open(path)
    }
    #[cfg(not(windows))]
    fs::File::open(path)
}

pub fn read(path: &Path) -> io::Result<Vec<u8>> {
    use io::Read;
    let mut bytes = Vec::new();
    open(path)?.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn retry<T>(mut attempt: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    let mut tries = 1;
    loop {
        match attempt() {
            Err(error) if tries < TRIES && error.kind() != io::ErrorKind::NotFound => {
                tries += 1;
                sleep(PAUSE);
            }
            result => return result,
        }
    }
}

/// Moves a finished temporary file over `path`.
pub fn persist(file: tempfile::NamedTempFile, path: &Path) -> io::Result<()> {
    let mut file = Some(file);
    retry(|| {
        let temp = file
            .take()
            .ok_or_else(|| io::Error::other("already persisted"))?;
        temp.persist(path).map(drop).map_err(|error| {
            let message = error.error.to_string();
            let kind = error.error.kind();
            file = Some(error.file);
            io::Error::new(kind, message)
        })
    })
}

pub fn remove(path: &Path) -> io::Result<()> {
    retry(|| fs::remove_file(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn a_file_held_open_by_a_reader_can_be_replaced_and_removed() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("status.json");
        fs::write(&path, b"old").unwrap();
        let reader = open(&path).unwrap();
        let mut temp = tempfile::NamedTempFile::new_in(directory.path()).unwrap();
        temp.write_all(b"new").unwrap();
        persist(temp, &path).unwrap();
        assert_eq!(read(&path).unwrap(), b"new");
        let reader2 = open(&path).unwrap();
        remove(&path).unwrap();
        drop((reader, reader2));
        assert!(!path.exists());
        assert_eq!(remove(&path).unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn retry_gives_up_after_a_few_tries() {
        let mut calls = 0;
        let result: io::Result<()> = retry(|| {
            calls += 1;
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        });
        assert!(result.is_err());
        assert_eq!(calls, TRIES);
    }
}
