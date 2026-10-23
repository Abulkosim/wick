use serde::Serialize;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    System,
    Display,
}

impl Kind {
    pub fn assert_type(self) -> &'static str {
        match self {
            Kind::System => "PreventUserIdleSystemSleep",
            Kind::Display => "PreventUserIdleDisplaySleep",
        }
    }
}

/// A sleep assertion held by another process, as powerd reports it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Held {
    pub pid: i32,
    pub process: String,
    pub assert_type: String,
    pub name: String,
    pub bundle_path: Option<String>,
    /// The process a system service asserts for, e.g. an app playing audio through coreaudiod.
    pub on_behalf_of: Option<i32>,
}

/// Released when dropped. powerd also releases it if Wick exits or crashes.
#[derive(Debug)]
pub struct Assertion {
    #[cfg_attr(not(any(target_os = "macos", windows)), allow(dead_code))]
    id: usize,
    pub kind: Kind,
}

#[cfg(target_os = "macos")]
mod ffi {
    use core_foundation::dictionary::CFDictionaryRef;

    #[link(name = "IOKit", kind = "framework")]
    extern "C" {
        pub fn IOPMAssertionCreateWithProperties(
            properties: CFDictionaryRef,
            assertion_id: *mut u32,
        ) -> i32;
        pub fn IOPMAssertionRelease(assertion_id: u32) -> i32;
        pub fn IOPMCopyAssertionsByProcess(assertions: *mut CFDictionaryRef) -> i32;
    }
}

#[cfg(target_os = "macos")]
pub fn create(kind: Kind, name: &str, timeout: Option<Duration>) -> Result<Assertion, String> {
    use core_foundation::{
        base::{CFType, TCFType},
        dictionary::CFDictionary,
        number::CFNumber,
        string::CFString,
    };

    let mut pairs: Vec<(CFString, CFType)> = vec![
        (
            CFString::new("AssertType"),
            CFString::new(kind.assert_type()).as_CFType(),
        ),
        (CFString::new("AssertName"), CFString::new(name).as_CFType()),
        (
            CFString::new("AssertLevel"),
            CFNumber::from(255).as_CFType(),
        ),
    ];
    if let Some(timeout) = timeout {
        let seconds = timeout.as_secs().clamp(1, i32::MAX as u64) as i32;
        pairs.push((
            CFString::new("TimeoutSeconds"),
            CFNumber::from(seconds).as_CFType(),
        ));
        pairs.push((
            CFString::new("TimeoutAction"),
            CFString::new("TimeoutActionRelease").as_CFType(),
        ));
    }
    let properties = CFDictionary::from_CFType_pairs(&pairs);
    let mut id = 0;
    let status = unsafe {
        ffi::IOPMAssertionCreateWithProperties(properties.as_concrete_TypeRef(), &mut id)
    };
    if status == 0 {
        Ok(Assertion {
            id: id as usize,
            kind,
        })
    } else {
        Err(format!(
            "macOS refused the sleep assertion (IOKit error {status:#x})."
        ))
    }
}

/// A power request: Windows keeps the system (and display) on while the handle is open, and
/// closes it when Wick exits.
#[cfg(windows)]
pub fn create(kind: Kind, name: &str, _timeout: Option<Duration>) -> Result<Assertion, String> {
    use super::win32::*;
    let reason = wide(name);
    let context = ReasonContext {
        version: POWER_REQUEST_CONTEXT_VERSION,
        flags: POWER_REQUEST_CONTEXT_SIMPLE_STRING,
        reason: [reason.as_ptr(), std::ptr::null(), std::ptr::null()],
    };
    unsafe {
        let request = PowerCreateRequest(&context);
        if request == INVALID_HANDLE_VALUE || request == 0 {
            return Err("Windows refused the power request.".into());
        }
        let mut ok = PowerSetRequest(request, POWER_REQUEST_SYSTEM_REQUIRED) != 0;
        if kind == Kind::Display {
            ok &= PowerSetRequest(request, POWER_REQUEST_DISPLAY_REQUIRED) != 0;
        }
        if !ok {
            CloseHandle(request);
            return Err("Windows refused the power request.".into());
        }
        Ok(Assertion {
            id: request as usize,
            kind,
        })
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
pub fn create(_kind: Kind, _name: &str, _timeout: Option<Duration>) -> Result<Assertion, String> {
    Err("Keeping awake isn't available on this system.".into())
}

impl Drop for Assertion {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        unsafe {
            ffi::IOPMAssertionRelease(self.id as u32);
        }
        #[cfg(windows)]
        unsafe {
            use super::win32::*;
            let request = self.id as HANDLE;
            PowerClearRequest(request, POWER_REQUEST_SYSTEM_REQUIRED);
            if self.kind == Kind::Display {
                PowerClearRequest(request, POWER_REQUEST_DISPLAY_REQUIRED);
            }
            CloseHandle(request);
        }
    }
}

#[cfg(target_os = "macos")]
pub fn list() -> Result<Vec<Held>, String> {
    use core_foundation::{
        array::CFArray,
        base::{CFType, TCFType},
        dictionary::{CFDictionary, CFDictionaryRef},
        number::CFNumber,
        string::CFString,
    };

    let mut raw: CFDictionaryRef = std::ptr::null();
    let status = unsafe { ffi::IOPMCopyAssertionsByProcess(&mut raw) };
    if status != 0 || raw.is_null() {
        return Err(format!(
            "Could not read power assertions (IOKit error {status:#x})."
        ));
    }
    let by_process: CFDictionary<CFNumber, CFArray<CFDictionary<CFString, CFType>>> =
        unsafe { CFDictionary::wrap_under_create_rule(raw) };
    let text = |entry: &CFDictionary<CFString, CFType>, key: &str| {
        entry
            .find(CFString::new(key))
            .and_then(|value| value.downcast::<CFString>())
            .map(|value| value.to_string())
    };
    let (pids, arrays) = by_process.get_keys_and_values();
    let mut held = Vec::new();
    for (pid, array) in pids.into_iter().zip(arrays) {
        let pid = unsafe { CFNumber::wrap_under_get_rule(pid.cast()) }
            .to_i32()
            .unwrap_or_default();
        let array: CFArray<CFDictionary<CFString, CFType>> =
            unsafe { CFArray::wrap_under_get_rule(array.cast()) };
        for entry in array.iter() {
            held.push(Held {
                pid,
                process: text(&entry, "Process Name").unwrap_or_default(),
                assert_type: text(&entry, "AssertType").unwrap_or_default(),
                name: text(&entry, "AssertName").unwrap_or_default(),
                bundle_path: text(&entry, "BundlePath"),
                on_behalf_of: entry
                    .find(CFString::new("AssertionOnBehalfOfPID"))
                    .and_then(|value| value.downcast::<CFNumber>())
                    .and_then(|value| value.to_i32()),
            });
        }
    }
    Ok(held)
}

#[cfg(not(target_os = "macos"))]
pub fn list() -> Result<Vec<Held>, String> {
    Ok(Vec::new())
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    fn ours(name: &str) -> Vec<Held> {
        let pid = std::process::id() as i32;
        list()
            .unwrap()
            .into_iter()
            .filter(|held| held.pid == pid && held.name == name)
            .collect()
    }

    #[test]
    fn assertion_appears_and_is_released_on_drop() {
        for kind in [Kind::System, Kind::Display] {
            let name = format!("Wick test {kind:?}");
            let assertion = create(kind, &name, Some(Duration::from_secs(30))).unwrap();
            let held = ours(&name);
            assert_eq!(held.len(), 1);
            assert_eq!(held[0].assert_type, kind.assert_type());
            drop(assertion);
            assert!(ours(&name).is_empty());
        }
    }
}
