#![allow(unsafe_code)]

use cap_std::fs::Dir;
use std::ffi::OsStr;
use std::fmt;
use std::fs::File;
use std::io;
use std::mem::{align_of, offset_of, size_of};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle};
use std::ptr::{addr_of_mut, null, null_mut};
use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
use windows_sys::Wdk::Storage::FileSystem::{
    FILE_CREATE, FILE_DIRECTORY_FILE, FILE_OPEN, FILE_OPEN_REPARSE_POINT,
    FILE_SYNCHRONOUS_IO_NONALERT, NtCreateFile,
};
use windows_sys::Win32::Foundation::{
    ERROR_ALREADY_EXISTS, ERROR_FILE_NOT_FOUND, ERROR_INVALID_HANDLE, ERROR_PATH_NOT_FOUND,
    GENERIC_READ, HANDLE, INVALID_HANDLE_VALUE, OBJ_CASE_INSENSITIVE, RtlNtStatusToDosError,
    STATUS_SUCCESS, UNICODE_STRING,
};
use windows_sys::Win32::Storage::FileSystem::{
    DELETE, FILE_ATTRIBUTE_DIRECTORY, FILE_DISPOSITION_INFO, FILE_READ_ATTRIBUTES,
    FILE_RENAME_INFO, FILE_RENAME_INFO_0, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    FileDispositionInfo, FileRenameInfo, SYNCHRONIZE, SetFileInformationByHandle,
};
use windows_sys::Win32::System::IO::{IO_STATUS_BLOCK, IO_STATUS_BLOCK_0};

const MAX_COMPONENT_UNITS: usize = 255;

/// The exact directory created by [`create_directory`].
///
/// This capability owns the single authoritative directory handle, its retained parent, and its
/// current parent-relative name. It is the only public source accepted for rename and removal, so
/// a regular file or a path-selected replacement cannot be supplied in its place.
pub struct OwnedDirectory {
    directory: Dir,
    parent: Dir,
    name: Vec<u16>,
}

impl fmt::Debug for OwnedDirectory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("OwnedDirectory").finish_non_exhaustive()
    }
}

impl OwnedDirectory {
    /// Borrows the exact directory for capability-relative file operations.
    #[must_use]
    pub fn directory(&self) -> &Dir {
        &self.directory
    }

    /// Renames this exact directory beneath `destination_parent` without replacement.
    ///
    /// The destination parent is cloned before mutation and becomes the retained parent only after
    /// the native rename succeeds. The source is never selected by path.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::InvalidInput`] for an invalid component, or the native error when
    /// Windows cannot clone the parent or rename the exact source handle.
    pub fn rename_noreplace(&mut self, destination_parent: &Dir, name: &OsStr) -> io::Result<()> {
        let name = encode_component(name)?;
        let retained_parent = destination_parent.try_clone()?;
        let mut buffer = RenameBuffer::new(raw_handle(destination_parent.as_handle()), &name)?;

        // SAFETY: `RenameBuffer` owns an initialized, ABI-aligned FILE_RENAME_INFO byte range of
        // the exact reported length. Both handles remain live throughout the synchronous call.
        // ReplaceIfExists is false and no source pathname or fallback is supplied.
        let succeeded = unsafe {
            SetFileInformationByHandle(
                raw_handle(self.directory.as_handle()),
                FileRenameInfo,
                buffer.as_mut_ptr().cast(),
                buffer.byte_len,
            )
        };
        if succeeded == 0 {
            return Err(io::Error::last_os_error());
        }
        self.parent = retained_parent;
        self.name = name;
        Ok(())
    }

    /// Removes this exact directory if it is empty, then confirms its name is absent.
    ///
    /// The directory is first marked for deletion through its authoritative handle. That handle is
    /// then closed, and the retained parent is used for a handle-relative, no-reparse open of the
    /// bound name. Success is reported only when Windows reports that name absent. A second handle
    /// that keeps deletion pending, or an object installed at the name, therefore returns an error.
    /// The consumed capability is closed on every outcome; an error after marking may mean deletion
    /// is still pending until another process closes its handle.
    ///
    /// # Errors
    ///
    /// Returns the native error if Windows cannot mark the exact empty directory for deletion, if
    /// deletion remains pending, or if absence cannot be confirmed unambiguously.
    pub fn remove_empty(self) -> io::Result<()> {
        let Self { directory, parent, name } = self;
        let source = directory.into_std_file();
        mark_for_deletion(&source)?;
        drop(source);
        confirm_absent(parent.as_handle(), &name)
    }
}

/// Atomically creates one directory relative to `parent` and returns its exact capability.
///
/// The authoritative handle has delete access and permits read/write sharing, but deliberately
/// excludes delete sharing. `name` must be one portable ASCII path component. At most 256 UTF-16
/// units are inspected and allocated while validating the 255-unit limit.
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidInput`] for an invalid component, or the mapped native error
/// when Windows cannot clone the parent, create the directory, or grant the required rights.
pub fn create_directory(parent: &Dir, name: &OsStr) -> io::Result<OwnedDirectory> {
    let name = encode_component(name)?;
    let retained_parent = parent.try_clone()?;
    let source = open_relative(
        parent.as_handle(),
        &name,
        GENERIC_READ | DELETE | SYNCHRONIZE,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        FILE_CREATE,
    )?;
    Ok(OwnedDirectory { directory: Dir::from_std_file(source), parent: retained_parent, name })
}

fn open_relative(
    parent: BorrowedHandle<'_>,
    name: &[u16],
    desired_access: u32,
    share_access: u32,
    disposition: u32,
) -> io::Result<File> {
    let name_bytes = utf16_byte_length_u16(name.len())?;
    let unicode_name = UNICODE_STRING {
        Length: name_bytes,
        MaximumLength: name_bytes,
        Buffer: name.as_ptr().cast_mut(),
    };
    let attributes = OBJECT_ATTRIBUTES {
        Length: u32::try_from(size_of::<OBJECT_ATTRIBUTES>()).map_err(invalid_input)?,
        RootDirectory: raw_handle(parent),
        ObjectName: &unicode_name,
        Attributes: OBJ_CASE_INSENSITIVE,
        SecurityDescriptor: null(),
        SecurityQualityOfService: null(),
    };
    let mut status_block =
        IO_STATUS_BLOCK { Anonymous: IO_STATUS_BLOCK_0 { Status: STATUS_SUCCESS }, Information: 0 };
    let mut handle: HANDLE = null_mut();

    // SAFETY: every pointer references a live, correctly aligned Windows ABI value for the
    // duration of the call. `parent` is borrowed, the name length is checked in bytes, and the
    // returned handle is converted to an owning Rust handle exactly once after successful return.
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            desired_access,
            &attributes,
            &mut status_block,
            null(),
            FILE_ATTRIBUTE_DIRECTORY,
            share_access,
            disposition,
            FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
            null(),
            0,
        )
    };
    if status < 0 {
        return Err(error_from_ntstatus(status));
    }
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::from_raw_os_error(ERROR_INVALID_HANDLE as i32));
    }

    // SAFETY: successful `NtCreateFile` returned one newly owned handle, checked above for both
    // invalid sentinel values. `OwnedHandle` establishes RAII before it is transferred to File.
    let owned = unsafe { OwnedHandle::from_raw_handle(handle) };
    Ok(File::from(owned))
}

fn mark_for_deletion(source: &File) -> io::Result<()> {
    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };

    // SAFETY: `disposition` is a live value with the exact Windows ABI type and length, and the
    // source File owns a valid handle for the duration of this synchronous call.
    let succeeded = unsafe {
        SetFileInformationByHandle(
            source.as_raw_handle().cast(),
            FileDispositionInfo,
            (&raw const disposition).cast(),
            u32::try_from(size_of::<FILE_DISPOSITION_INFO>()).map_err(invalid_input)?,
        )
    };
    if succeeded == 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
}

fn confirm_absent(parent: BorrowedHandle<'_>, name: &[u16]) -> io::Result<()> {
    match open_relative(
        parent,
        name,
        FILE_READ_ATTRIBUTES | SYNCHRONIZE,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        FILE_OPEN,
    ) {
        Ok(found) => {
            drop(found);
            Err(io::Error::from_raw_os_error(ERROR_ALREADY_EXISTS as i32))
        }
        Err(error)
            if matches!(error.raw_os_error(), Some(code)
                if code == ERROR_FILE_NOT_FOUND as i32 || code == ERROR_PATH_NOT_FOUND as i32) =>
        {
            Ok(())
        }
        Err(error) => Err(error),
    }
}

struct RenameBuffer {
    words: Vec<usize>,
    byte_len: u32,
}

impl RenameBuffer {
    fn new(destination_parent: HANDLE, name: &[u16]) -> io::Result<Self> {
        const {
            assert!(align_of::<usize>() >= align_of::<FILE_RENAME_INFO>());
        }

        let header_bytes = offset_of!(FILE_RENAME_INFO, FileName);
        let name_bytes = name
            .len()
            .checked_mul(size_of::<u16>())
            .ok_or_else(|| invalid_input("directory component length overflow"))?;
        let byte_len = header_bytes
            .checked_add(name_bytes)
            .ok_or_else(|| invalid_input("rename buffer length overflow"))?;
        let word_count = byte_len
            .checked_add(size_of::<usize>() - 1)
            .ok_or_else(|| invalid_input("rename buffer allocation overflow"))?
            / size_of::<usize>();
        let mut buffer = Self {
            words: vec![0; word_count],
            byte_len: u32::try_from(byte_len).map_err(invalid_input)?,
        };
        let info = buffer.as_mut_ptr();

        // SAFETY: the zeroed usize allocation has at least `byte_len` initialized bytes and the
        // const assertion proves sufficient FILE_RENAME_INFO alignment. Each field and the exact
        // trailing UTF-16 byte range lie within that allocation.
        unsafe {
            (*info).Anonymous = FILE_RENAME_INFO_0 { ReplaceIfExists: false };
            (*info).RootDirectory = destination_parent;
            (*info).FileNameLength = u32::try_from(name_bytes).map_err(invalid_input)?;
            std::ptr::copy_nonoverlapping(
                name.as_ptr(),
                addr_of_mut!((*info).FileName).cast::<u16>(),
                name.len(),
            );
        }
        Ok(buffer)
    }

    fn as_mut_ptr(&mut self) -> *mut FILE_RENAME_INFO {
        self.words.as_mut_ptr().cast()
    }
}

fn raw_handle(handle: BorrowedHandle<'_>) -> HANDLE {
    handle.as_raw_handle().cast()
}

fn error_from_ntstatus(status: i32) -> io::Error {
    // SAFETY: RtlNtStatusToDosError accepts every NTSTATUS value and returns a Win32 error code.
    let code = unsafe { RtlNtStatusToDosError(status) };
    let code = i32::try_from(code).unwrap_or(i32::MAX);
    io::Error::from_raw_os_error(code)
}

fn encode_component(name: &OsStr) -> io::Result<Vec<u16>> {
    let units: Vec<u16> = name.encode_wide().take(MAX_COMPONENT_UNITS + 1).collect();
    if units.is_empty() || units.len() > MAX_COMPONENT_UNITS {
        return Err(invalid_input("directory name must contain 1 to 255 UTF-16 units"));
    }
    let bytes: Vec<u8> = units
        .iter()
        .copied()
        .map(u8::try_from)
        .collect::<Result<_, _>>()
        .map_err(|_| invalid_input("directory name must be portable ASCII"))?;
    if bytes == b"." || bytes == b".." {
        return Err(invalid_input("dot directory components are not permitted"));
    }
    if bytes
        .iter()
        .any(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')))
        || bytes.last() == Some(&b'.')
    {
        return Err(invalid_input("directory name is not a portable ASCII component"));
    }
    let stem = bytes.split(|byte| *byte == b'.').next().unwrap_or(&bytes);
    if is_reserved_device_stem(stem) {
        return Err(invalid_input("reserved Windows device name is not permitted"));
    }
    Ok(units)
}

fn is_reserved_device_stem(stem: &[u8]) -> bool {
    [b"CON".as_slice(), b"PRN", b"AUX", b"NUL"]
        .iter()
        .any(|reserved| stem.eq_ignore_ascii_case(reserved))
        || ((stem.len() == 4)
            && (stem[..3].eq_ignore_ascii_case(b"COM") || stem[..3].eq_ignore_ascii_case(b"LPT"))
            && stem[3].is_ascii_digit())
}

fn utf16_byte_length_u16(units: usize) -> io::Result<u16> {
    units
        .checked_mul(size_of::<u16>())
        .and_then(|bytes| u16::try_from(bytes).ok())
        .ok_or_else(|| invalid_input("directory component byte length overflow"))
}

fn invalid_input(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error.to_string())
}
