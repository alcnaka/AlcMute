//! Two auto-reset events, restricted by DACL to this token's user SID.
//! No polling in the resident process; the one listener sleeps in the kernel.
use super::current_user_id;
use crate::Result;
use std::{
    sync::Arc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, LocalFree, HANDLE, WAIT_OBJECT_0},
    Security::{
        Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW, SECURITY_ATTRIBUTES,
    },
    System::Threading::{
        CreateEventW, OpenEventW, SetEvent, WaitForMultipleObjects, WaitForSingleObject,
        EVENT_MODIFY_STATE, SYNCHRONIZATION_SYNCHRONIZE,
    },
};

struct Event(usize);
impl Event {
    fn handle(&self) -> HANDLE {
        self.0 as HANDLE
    }
    fn signal(&self) {
        unsafe {
            SetEvent(self.handle());
        }
    }
}
impl Drop for Event {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle());
        }
    }
}
fn name(suffix: &str) -> Result<Vec<u16>> {
    let suffix = if cfg!(test) {
        format!("Tests.{}.{}", std::process::id(), suffix)
    } else {
        suffix.to_owned()
    };
    Ok(
        format!("Local\\Alcnaka.AlcMute.{}.{}", current_user_id()?, suffix)
            .encode_utf16()
            .chain(Some(0))
            .collect(),
    )
}
pub(super) fn sid_string() -> Result<String> {
    let hex = current_user_id()?;
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>>>()?;
    if bytes.len() < 8 {
        return Err("Invalid user SID".into());
    }
    let authority = bytes[2..8]
        .iter()
        .fold(0u64, |value, b| (value << 8) | u64::from(*b));
    let mut result = format!("S-{}-{authority}", bytes[0]);
    for sub in bytes[8..].as_chunks::<4>().0 {
        result.push_str(&format!("-{}", u32::from_le_bytes(*sub)));
    }
    Ok(result)
}
fn event(suffix: &str) -> Result<Arc<Event>> {
    secured_object(suffix, |attributes, name| unsafe {
        CreateEventW(attributes, 0, 0, name)
    })
}
fn secured_object(
    suffix: &str,
    create: impl FnOnce(*const SECURITY_ATTRIBUTES, *const u16) -> HANDLE,
) -> Result<Arc<Event>> {
    let object_name = name(suffix)?;
    let sddl: Vec<u16> = format!("D:P(A;;GA;;;{})", sid_string()?)
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut descriptor = std::ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    let handle = create(&attributes, object_name.as_ptr());
    unsafe {
        LocalFree(descriptor);
    }
    if handle.is_null() {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(Arc::new(Event(handle as usize)))
}
#[derive(Clone)]
pub struct Reply(Arc<Event>);
impl Reply {
    pub fn acknowledge(&self) {
        self.0.signal();
    }
}
pub struct ActivationServer {
    _owner: Arc<Event>,
    stop: Arc<Event>,
    worker: Option<JoinHandle<()>>,
}
impl ActivationServer {
    pub fn start(callback: impl Fn(Reply) + Send + 'static) -> Result<Self> {
        use windows_sys::Win32::{
            Foundation::INVALID_HANDLE_VALUE,
            System::Memory::{
                CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, FILE_MAP_WRITE, PAGE_READWRITE,
            },
        };
        let owner = secured_object("ActivationOwner", |attributes, name| unsafe {
            CreateFileMappingW(INVALID_HANDLE_VALUE, attributes, PAGE_READWRITE, 0, 4, name)
        })?;
        let view = unsafe { MapViewOfFile(owner.handle(), FILE_MAP_WRITE, 0, 0, 4) };
        if view.Value.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        unsafe {
            std::ptr::write_volatile(view.Value.cast::<u32>(), std::process::id());
            UnmapViewOfFile(view);
        }
        let request = event("ShowSettings")?;
        let ack = event("ShowSettingsAck")?;
        let stop = event("StopActivation")?;
        let worker_stop = stop.clone();
        let worker = thread::spawn(move || {
            let handles = [worker_stop.handle(), request.handle()];
            loop {
                let result = unsafe { WaitForMultipleObjects(2, handles.as_ptr(), 0, u32::MAX) };
                if result != WAIT_OBJECT_0 + 1 {
                    break;
                }
                callback(Reply(ack.clone()));
            }
        });
        Ok(Self {
            _owner: owner,
            stop,
            worker: Some(worker),
        })
    }
}
impl Drop for ActivationServer {
    fn drop(&mut self) {
        self.stop.signal();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn open(suffix: &str) -> Result<Event> {
    let handle = unsafe {
        OpenEventW(
            EVENT_MODIFY_STATE | SYNCHRONIZATION_SYNCHRONIZE,
            0,
            name(suffix)?.as_ptr(),
        )
    };
    if handle.is_null() {
        Err(std::io::Error::last_os_error().to_string())
    } else {
        Ok(Event(handle as usize))
    }
}
pub fn request_settings() -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(3);
    let (request, ack) = loop {
        if let (Ok(request), Ok(ack)) = (open("ShowSettings"), open("ShowSettingsAck")) {
            break (request, ack);
        }
        if Instant::now() >= deadline {
            return Err("起動済みのAlcMuteが応答しません。トレイから設定を開くか、終了して再起動してください。".into());
        }
        thread::sleep(Duration::from_millis(25));
    };
    // Transfer foreground permission even before the lazy window exists.
    // The PID is published in a mapping with the same current-user-only DACL.
    unsafe {
        use windows_sys::Win32::{
            System::Memory::{MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, FILE_MAP_READ},
            UI::WindowsAndMessaging::AllowSetForegroundWindow,
        };
        let raw = OpenFileMappingW(FILE_MAP_READ, 0, name("ActivationOwner")?.as_ptr());
        if raw.is_null() {
            return Err("Activation owner is unavailable".into());
        }
        let mapping = Event(raw as usize);
        let view = MapViewOfFile(mapping.handle(), FILE_MAP_READ, 0, 0, 4);
        if view.Value.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let pid = std::ptr::read_volatile(view.Value.cast::<u32>());
        UnmapViewOfFile(view);
        if pid == 0 {
            return Err("Invalid activation owner".into());
        }
        AllowSetForegroundWindow(pid);
    }
    unsafe {
        windows_sys::Win32::System::Threading::ResetEvent(ack.handle());
    }
    request.signal();
    if unsafe { WaitForSingleObject(ack.handle(), 5000) } != WAIT_OBJECT_0 {
        return Err("設定画面の表示要求がタイムアウトしました".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn activation_round_trip_and_listener_shutdown() {
        let (tx, rx) = std::sync::mpsc::channel();
        let server = ActivationServer::start(move |reply| {
            tx.send(()).unwrap();
            reply.acknowledge();
        })
        .unwrap();
        request_settings().unwrap();
        rx.recv_timeout(Duration::from_secs(1)).unwrap();
        let now = Instant::now();
        drop(server);
        assert!(now.elapsed() < Duration::from_secs(1));
    }
}
