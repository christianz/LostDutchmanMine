//! DOS files: the game folder is read-only; every write goes to the saves folder.
//!
//! A file opened for writing that exists only in the game folder is first copied
//! into the saves folder, so the player's original saves are never modified.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};

use machine::Machine;
use machine::state::{Reader, StateError, Writer};

use crate::services::{failure, read_string, success};
use crate::{Dos, DosError};

/// The first handle DOS gives a program's files; 0-4 are the standard devices.
const FIRST_HANDLE: u16 = 5;

/// DOS error codes the file services report.
const FILE_NOT_FOUND: u16 = 2;
const INVALID_FUNCTION: u16 = 1;
const ACCESS_DENIED: u16 = 5;
const INVALID_HANDLE: u16 = 6;

/// Open files and the two folders names resolve in.
#[derive(Debug)]
pub struct Files {
    data: PathBuf,
    saves: PathBuf,
    open: BTreeMap<u16, Open>,
}

/// An open file, and what reopens it from a savestate.
#[derive(Debug)]
struct Open {
    file: File,
    path: PathBuf,
    write: bool,
}

/// Which folder a saved open file is in.
const IN_SAVES: u8 = 0;
const IN_DATA: u8 = 1;
const ELSEWHERE: u8 = 2;

impl Files {
    /// Files read from `data`, written to `saves`.
    pub fn new(data: PathBuf, saves: PathBuf) -> Self {
        Files { data, saves, open: BTreeMap::new() }
    }

    /// The saves folder.
    pub fn saves(&self) -> &Path {
        &self.saves
    }

    /// Where a DOS file name lives: in the saves folder when writing or when
    /// it already exists there, otherwise in the game folder. Each component
    /// matches case-insensitively, as on DOS.
    ///
    /// # Errors
    ///
    /// [`DosError::ParentPath`] for names containing `..`.
    pub fn resolve(&self, name: &[u8], write: bool) -> Result<PathBuf, DosError> {
        let text: String =
            name.iter().map(|&byte| if byte == b'\\' { '/' } else { char::from(byte) }).collect();
        let text = if text.as_bytes().get(1) == Some(&b':') { &text[2..] } else { &text[..] };
        let relative = Path::new(text.trim_start_matches('/'));
        if relative.components().any(|part| part == Component::ParentDir) {
            return Err(DosError::ParentPath(text.to_owned()));
        }
        let saved = find(&self.saves, relative);
        if write || saved.exists() { Ok(saved) } else { Ok(find(&self.data, relative)) }
    }

    fn get(&mut self, handle: u16) -> Option<&mut File> {
        self.open.get_mut(&handle).map(|open| &mut open.file)
    }

    fn insert(&mut self, file: File, path: PathBuf, write: bool) -> u16 {
        let handle = (FIRST_HANDLE..=u16::MAX)
            .find(|handle| !self.open.contains_key(handle))
            .expect("a free handle");
        self.open.insert(handle, Open { file, path, write });
        handle
    }

    /// Each open file by folder, name within it, mode and position, so a
    /// state restores into another session's folders.
    pub(crate) fn save(&self, w: &mut Writer) {
        w.u16(self.open.len() as u16);
        for (&handle, open) in &self.open {
            let (folder, name) = if let Ok(name) = open.path.strip_prefix(&self.saves) {
                (IN_SAVES, name)
            } else if let Ok(name) = open.path.strip_prefix(&self.data) {
                (IN_DATA, name)
            } else {
                (ELSEWHERE, open.path.as_path())
            };
            w.u16(handle);
            w.u8(folder);
            w.bytes(name.to_string_lossy().as_bytes());
            w.bool(open.write);
            w.u64((&open.file).stream_position().unwrap_or(0));
        }
    }

    /// Reopens the files a state had open, at their positions.
    pub(crate) fn restore(&mut self, r: &mut Reader) -> Result<(), StateError> {
        self.open.clear();
        for _ in 0..r.u16()? {
            let handle = r.u16()?;
            let folder = r.u8()?;
            let name = String::from_utf8_lossy(r.bytes()?).into_owned();
            let (write, position) = (r.bool()?, r.u64()?);
            let path = match folder {
                IN_SAVES => self.saves.join(&name),
                IN_DATA => self.data.join(&name),
                ELSEWHERE => PathBuf::from(&name),
                other => return Err(StateError::Invalid(format!("folder {other} of {name}"))),
            };
            let reopen = || -> std::io::Result<File> {
                let mut file = OpenOptions::new().read(true).write(write).open(&path)?;
                file.seek(SeekFrom::Start(position))?;
                Ok(file)
            };
            let file = reopen()
                .map_err(|error| StateError::Invalid(format!("{}: {error}", path.display())))?;
            self.open.insert(handle, Open { file, path, write });
        }
        Ok(())
    }
}

/// Resolves `relative` under `root`, matching each missing component to a
/// directory entry that differs only in case.
fn find(root: &Path, relative: &Path) -> PathBuf {
    let mut path = root.to_path_buf();
    for part in relative.components() {
        let wanted = part.as_os_str().to_string_lossy().to_lowercase();
        let exact = path.join(part);
        let matched = (!exact.exists() && path.is_dir())
            .then(|| fs::read_dir(&path).ok())
            .flatten()
            .and_then(|entries| {
                entries.flatten().map(|entry| entry.path()).find(|candidate| {
                    candidate
                        .file_name()
                        .is_some_and(|name| name.to_string_lossy().to_lowercase() == wanted)
                })
            });
        path = matched.unwrap_or(exact);
    }
    path
}

fn io(path: &Path, source: std::io::Error) -> DosError {
    DosError::Io { path: path.to_path_buf(), source }
}

impl Dos {
    /// INT 21h file calls: 3Ch-40h, 42h and 43h.
    pub(crate) fn file_service(&mut self, m: &mut Machine) -> Result<bool, DosError> {
        match m.regs.ah() {
            0x3c | 0x3d => self.open(m)?,
            0x3e => match self.files.open.remove(&m.regs.bx) {
                Some(_) => success(m),
                None => failure(m, INVALID_HANDLE),
            },
            0x3f => self.read(m),
            0x40 => self.write(m),
            0x42 => self.seek(m),
            0x43 => self.attributes(m)?,
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// 3Ch creates (truncating); 3Dh opens, for writing when AL's access bits say so.
    fn open(&mut self, m: &mut Machine) -> Result<(), DosError> {
        let create = m.regs.ah() == 0x3c;
        let write = create || m.regs.al() & 3 != 0;
        let name = read_string(m, m.regs.ds, m.regs.dx, 0)?;
        let path = self.files.resolve(&name, write)?;
        if write {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| io(parent, error))?;
            }
            if !create && !path.exists() {
                let original = self.files.resolve(&name, false)?;
                if original.is_file() {
                    fs::copy(&original, &path).map_err(|error| io(&original, error))?;
                }
            }
        }
        let mut options = OpenOptions::new();
        options.read(true).write(write).create(create).truncate(create);
        match options.open(&path) {
            Ok(file) => {
                m.regs.ax = self.files.insert(file, path, write);
                success(m);
            }
            Err(_) => failure(m, FILE_NOT_FOUND),
        }
        Ok(())
    }

    /// 3Fh: reads up to CX bytes into DS:DX; AX is the count read.
    fn read(&mut self, m: &mut Machine) {
        let wanted = usize::from(m.regs.cx);
        let Some(file) = self.files.get(m.regs.bx) else {
            return failure(m, INVALID_HANDLE);
        };
        let mut buffer = vec![0; wanted];
        let mut filled = 0;
        while filled < wanted {
            match file.read(&mut buffer[filled..]) {
                Ok(0) => break,
                Ok(count) => filled += count,
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(_) => return failure(m, ACCESS_DENIED),
            }
        }
        for (i, &byte) in buffer[..filled].iter().enumerate() {
            m.memory.write8(m.regs.ds, m.regs.dx.wrapping_add(i as u16), byte);
        }
        m.regs.ax = filled as u16;
        success(m);
    }

    /// 40h: writes CX bytes from DS:DX; handles 1 and 2 are the console.
    fn write(&mut self, m: &mut Machine) {
        let bytes: Vec<u8> =
            (0..m.regs.cx).map(|i| m.memory.read8(m.regs.ds, m.regs.dx.wrapping_add(i))).collect();
        if matches!(m.regs.bx, 1 | 2) {
            self.console.extend(&bytes);
            m.regs.ax = m.regs.cx;
            return success(m);
        }
        let Some(file) = self.files.get(m.regs.bx) else {
            return failure(m, INVALID_HANDLE);
        };
        m.regs.ax = file.write_all(&bytes).map_or(0, |()| m.regs.cx);
        success(m);
    }

    /// 42h: moves the file pointer; DX:AX is the new position.
    fn seek(&mut self, m: &mut Machine) {
        let offset = i64::from(((u32::from(m.regs.cx) << 16) | u32::from(m.regs.dx)) as i32);
        let whence = m.regs.al();
        let Some(file) = self.files.get(m.regs.bx) else {
            return failure(m, INVALID_HANDLE);
        };
        let target = match whence {
            0 if offset >= 0 => SeekFrom::Start(offset as u64),
            1 => SeekFrom::Current(offset),
            2 => SeekFrom::End(offset),
            _ => return failure(m, INVALID_FUNCTION),
        };
        match file.seek(target) {
            Ok(position) => {
                (m.regs.ax, m.regs.dx) = (position as u16, (position >> 16) as u16);
                success(m);
            }
            Err(_) => failure(m, INVALID_FUNCTION),
        }
    }

    /// 43h AL=00h: a file's attributes, all clear.
    fn attributes(&mut self, m: &mut Machine) -> Result<(), DosError> {
        if m.regs.al() != 0 {
            failure(m, INVALID_FUNCTION);
            return Ok(());
        }
        let name = read_string(m, m.regs.ds, m.regs.dx, 0)?;
        if self.files.resolve(&name, false)?.exists() {
            m.regs.cx = 0;
            success(m);
        } else {
            failure(m, FILE_NOT_FOUND);
        }
        Ok(())
    }
}
