use crate::line::{LogLine, parse_line};

const FRAME_BYTES: u32 = 4096;
pub const FRAME_SIZE: usize = FRAME_BYTES as usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugMessage {
    pub pid: u32,
    pub text: String,
}

pub fn parse_frame(frame: &[u8]) -> Option<DebugMessage> {
    let (pid, text) = frame.split_at_checked(4)?;
    let pid = u32::from_le_bytes(pid.try_into().ok()?);
    let end = memchr::memchr(0, text).unwrap_or(text.len());
    Some(DebugMessage {
        pid,
        text: String::from_utf8_lossy(&text[..end])
            .trim_end_matches(['\n', '\r'])
            .to_owned(),
    })
}

pub fn line_from_frame(frame: &[u8], pid: u32) -> Option<LogLine> {
    let message = parse_frame(frame)?;
    if message.pid != pid {
        return None;
    }
    parse_line(&message.text)
}

#[cfg(windows)]
mod channel {
    use windows_sys::Win32::Foundation::{
        CloseHandle, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0,
    };
    use windows_sys::Win32::System::Memory::{
        CreateFileMappingW, FILE_MAP_READ, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile,
        PAGE_READWRITE, UnmapViewOfFile,
    };
    use windows_sys::Win32::System::Threading::{
        CreateEventW, OpenProcess, PROCESS_SYNCHRONIZE, SetEvent, WaitForMultipleObjects,
        WaitForSingleObject,
    };

    use crate::error::{LogError, Result};
    use crate::line::LogLine;

    use super::{FRAME_BYTES, FRAME_SIZE, line_from_frame};

    const fn wide<const N: usize>(text: &str) -> [u16; N] {
        let bytes = text.as_bytes();
        let mut wide = [0u16; N];
        let mut at = 0;
        while at < bytes.len() {
            wide[at] = bytes[at] as u16;
            at += 1;
        }
        wide
    }

    const BUFFER_NAME: [u16; 19] = wide("Local\\DBWIN_BUFFER");
    const BUFFER_READY_NAME: [u16; 25] = wide("Local\\DBWIN_BUFFER_READY");
    const DATA_READY_NAME: [u16; 23] = wide("Local\\DBWIN_DATA_READY");
    const WAIT_MILLIS: u32 = 1000;

    fn failed(what: &str) -> LogError {
        LogError::DebugChannel(std::io::Error::other(format!(
            "{what}: {}",
            std::io::Error::last_os_error()
        )))
    }

    struct Owned(HANDLE);

    impl Drop for Owned {
        fn drop(&mut self) {
            // SAFETY: the handle came from a successful CreateEventW or OpenProcess and is closed once.
            unsafe { CloseHandle(self.0) };
        }
    }

    struct Frames {
        section: HANDLE,
        view: MEMORY_MAPPED_VIEW_ADDRESS,
    }

    impl Drop for Frames {
        fn drop(&mut self) {
            // SAFETY: the view and the section came from successful MapViewOfFile and
            // SAFETY: CreateFileMappingW calls and are each released once.
            unsafe {
                UnmapViewOfFile(self.view);
                CloseHandle(self.section);
            }
        }
    }

    fn map_frames() -> Result<Frames> {
        // SAFETY: INVALID_HANDLE_VALUE asks for a pagefile-backed section, the name is a
        // SAFETY: the name is NUL-terminated and FRAME_SIZE fits a u32.
        let section = unsafe {
            CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                std::ptr::null(),
                PAGE_READWRITE,
                0,
                FRAME_BYTES,
                BUFFER_NAME.as_ptr(),
            )
        };
        if section.is_null() {
            return Err(failed("Creating DBWIN_BUFFER"));
        }
        // SAFETY: the section handle is live and FRAME_SIZE is the size it was created with.
        let view = unsafe { MapViewOfFile(section, FILE_MAP_READ, 0, 0, FRAME_SIZE) };
        if view.Value.is_null() {
            let error = failed("Mapping DBWIN_BUFFER");
            // SAFETY: the section handle is live and is closed once here.
            unsafe { CloseHandle(section) };
            return Err(error);
        }
        Ok(Frames { section, view })
    }

    fn open_game(pid: u32) -> Result<Owned> {
        // SAFETY: OpenProcess takes only by-value arguments and returns an owned handle or null.
        let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
        if handle.is_null() {
            return Err(failed("Opening the game process"));
        }
        Ok(Owned(handle))
    }

    fn create_event(name: &[u16]) -> Result<Owned> {
        // SAFETY: the name is a NUL-terminated wide string and a null descriptor is allowed.
        let handle = unsafe { CreateEventW(std::ptr::null(), 0, 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(failed("Creating a DBWIN event"));
        }
        Ok(Owned(handle))
    }

    pub struct DbgWinListener {
        pid: u32,
        game: Owned,
        frames: Frames,
        buffer_ready: Owned,
        data_ready: Owned,
    }

    // SAFETY: the section, the view and both events are process-wide kernel objects that any
    // SAFETY: thread may use, and the listener owns them exclusively.
    unsafe impl Send for DbgWinListener {}

    impl DbgWinListener {
        pub fn open(pid: u32) -> Result<Self> {
            let listener = Self {
                pid,
                game: open_game(pid)?,
                frames: map_frames()?,
                buffer_ready: create_event(&BUFFER_READY_NAME)?,
                data_ready: create_event(&DATA_READY_NAME)?,
            };
            listener.release_buffer();
            Ok(listener)
        }

        fn release_buffer(&self) {
            // SAFETY: the handle is a live auto-reset event owned by this listener.
            unsafe { SetEvent(self.buffer_ready.0) };
        }

        pub fn game_exited(&self) -> bool {
            // SAFETY: the process handle is live and was opened with SYNCHRONIZE.
            unsafe { WaitForSingleObject(self.game.0, 0) == WAIT_OBJECT_0 }
        }

        pub fn next_line(&mut self) -> Option<LogLine> {
            let handles = [self.data_ready.0, self.game.0];
            // SAFETY: both handles are live for as long as the listener, and the array holds two.
            let signalled = unsafe { WaitForMultipleObjects(2, handles.as_ptr(), 0, WAIT_MILLIS) };
            if signalled != WAIT_OBJECT_0 {
                return None;
            }
            let mut frame = [0u8; FRAME_SIZE];
            // SAFETY: the view maps FRAME_SIZE readable bytes for as long as `frames` lives, and
            // SAFETY: the destination is exactly FRAME_SIZE bytes long.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    self.frames.view.Value.cast::<u8>(),
                    frame.as_mut_ptr(),
                    FRAME_SIZE,
                );
            }
            self.release_buffer();
            line_from_frame(&frame, self.pid)
        }
    }
}

#[cfg(windows)]
pub use channel::DbgWinListener;

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(pid: u32, text: &[u8]) -> Vec<u8> {
        let mut frame = pid.to_le_bytes().to_vec();
        frame.extend_from_slice(text);
        frame.push(0);
        frame.resize(FRAME_SIZE, 0);
        frame
    }

    #[test]
    fn pid_and_text() {
        let bytes = frame(4242, b"1041.984 Script [Info]: WaveDefend.lua: spawn");
        assert_eq!(
            parse_frame(&bytes),
            Some(DebugMessage {
                pid: 4242,
                text: "1041.984 Script [Info]: WaveDefend.lua: spawn".to_owned(),
            })
        );
    }

    #[test]
    fn trailing_line_breaks() {
        let bytes = frame(7, b"1.000 Sys [Info]: with a break\r\n");
        assert_eq!(
            parse_frame(&bytes).unwrap().text,
            "1.000 Sys [Info]: with a break"
        );
    }

    #[test]
    fn unterminated_text() {
        let mut bytes = 9u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(b"2.000 Sys [Info]: unterminated");
        assert_eq!(
            parse_frame(&bytes).unwrap().text,
            "2.000 Sys [Info]: unterminated"
        );
    }

    #[test]
    fn short_frame() {
        assert_eq!(parse_frame(&[1, 2, 3]), None);
        assert_eq!(parse_frame(&[]), None);
    }

    #[test]
    fn game_pid_only() {
        let bytes = frame(4242, b"3.000 Sys [Info]: from the game");
        let line = line_from_frame(&bytes, 4242).unwrap();
        assert_eq!(line.message, "from the game");
        assert_eq!(line_from_frame(&bytes, 99), None);
    }

    #[test]
    fn non_log_frame() {
        let bytes = frame(4242, b"HEAP: reallocating a block");
        assert_eq!(line_from_frame(&bytes, 4242), None);
    }

    #[cfg(windows)]
    #[test]
    fn synthetic_frame_on_windows() {
        let bytes = frame(1234, b"4.000 Net [Info]: NAT bound for client");
        let message = parse_frame(&bytes).unwrap();
        assert_eq!(message.pid, 1234);
        let line = line_from_frame(&bytes, 1234).unwrap();
        assert_eq!(line.message, "NAT bound for client");
    }

    #[cfg(windows)]
    #[test]
    fn listeners_share_channel() {
        let pid = std::process::id();
        let first = super::DbgWinListener::open(pid).unwrap();
        assert!(super::DbgWinListener::open(pid).is_ok());
        drop(first);
    }

    #[cfg(windows)]
    #[test]
    fn listener_sees_process_exit() {
        let own = super::DbgWinListener::open(std::process::id()).unwrap();
        assert!(!own.game_exited());
        drop(own);
        let mut child = std::process::Command::new("cmd")
            .args(["/c", "exit"])
            .spawn()
            .unwrap();
        let listener = super::DbgWinListener::open(child.id()).unwrap();
        child.wait().unwrap();
        assert!(listener.game_exited());
    }
}
