#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case)]

#[cfg(feature = "link")]
use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
};

pub const CVC5_NAME: &str = "CVC5";
pub const CVC5_VERSION: &str = "1.3.3";
pub const CVC5_TAG: &str = "cvc5-1.3.3";
pub const CVC5_SOURCE_URL: &str = "https://github.com/cvc5/cvc5";
pub const CVC5_EXPECTED_COMMIT: &str = "8ff882e3e42f046867d2ac2e33e92b3d026144ae";

pub fn cvc5_source_mode() -> &'static str {
    option_env!("SOTER_CVC5_SOURCE_MODE").unwrap_or("native-link-disabled")
}

pub fn cvc5_source_commit() -> &'static str {
    option_env!("SOTER_CVC5_SOURCE_COMMIT").unwrap_or("native-link-disabled")
}

pub fn cvc5_configure_flags() -> &'static str {
    option_env!("SOTER_CVC5_CONFIGURE_FLAGS").unwrap_or("--no-poly")
}

#[cfg(feature = "link")]
unsafe extern "C" {
    fn soter_cvc5_version() -> *const c_char;
    fn soter_cvc5_check_sat_smt2(
        smtlib: *const c_char,
        timeout_ms: u64,
        produce_model: bool,
        produce_unsat_core: bool,
    ) -> RawSolveResult;
    fn soter_cvc5_solve_result_delete(result: *mut RawSolveResult);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cvc5Status {
    Sat,
    Unsat,
    Unknown,
    Timeout,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cvc5SolveReport {
    pub status: Cvc5Status,
    pub model: Option<String>,
    pub unsat_core: Option<String>,
    pub diagnostics: Option<String>,
}

#[cfg(feature = "link")]
#[repr(C)]
struct RawSolveResult {
    status: i32,
    model: *mut c_char,
    unsat_core: *mut c_char,
    diagnostics: *mut c_char,
}

/// Return the linked CVC5 version string.
#[cfg(feature = "link")]
pub fn linked_version() -> String {
    // SAFETY: `soter_cvc5_version` returns a pointer to a static string owned
    // by the wrapper. The wrapper keeps it valid until process exit.
    unsafe {
        CStr::from_ptr(soter_cvc5_version())
            .to_string_lossy()
            .into_owned()
    }
}

#[cfg(feature = "link")]
pub fn check_sat_smt2(
    smtlib: &str,
    timeout_ms: u64,
    produce_model: bool,
    produce_unsat_core: bool,
) -> Cvc5SolveReport {
    let smtlib = match CString::new(smtlib) {
        Ok(smtlib) => smtlib,
        Err(err) => {
            return Cvc5SolveReport {
                status: Cvc5Status::Error,
                model: None,
                unsat_core: None,
                diagnostics: Some(format!("SMT-LIB input contains interior NUL byte: {err}")),
            };
        }
    };

    // SAFETY: the wrapper copies any returned strings into owned allocations.
    // `soter_cvc5_solve_result_delete` frees those allocations after Rust has
    // copied them into owned `String`s.
    unsafe {
        let mut raw = soter_cvc5_check_sat_smt2(
            smtlib.as_ptr(),
            timeout_ms,
            produce_model,
            produce_unsat_core,
        );
        let report = Cvc5SolveReport {
            status: map_status(raw.status),
            model: take_string(raw.model),
            unsat_core: take_string(raw.unsat_core),
            diagnostics: take_string(raw.diagnostics),
        };
        soter_cvc5_solve_result_delete(std::ptr::addr_of_mut!(raw));
        report
    }
}

#[cfg(not(feature = "link"))]
pub fn linked_version() -> &'static str {
    "cvc5 native link feature disabled"
}

#[cfg(not(feature = "link"))]
pub fn check_sat_smt2(
    _smtlib: &str,
    _timeout_ms: u64,
    _produce_model: bool,
    _produce_unsat_core: bool,
) -> Cvc5SolveReport {
    Cvc5SolveReport {
        status: Cvc5Status::Error,
        model: None,
        unsat_core: None,
        diagnostics: Some("cvc5 native link feature disabled".to_string()),
    }
}

#[cfg(feature = "link")]
fn map_status(status: i32) -> Cvc5Status {
    match status {
        0 => Cvc5Status::Sat,
        1 => Cvc5Status::Unsat,
        2 => Cvc5Status::Unknown,
        3 => Cvc5Status::Timeout,
        _ => Cvc5Status::Error,
    }
}

#[cfg(feature = "link")]
unsafe fn take_string(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }

    // SAFETY: `ptr` is a NUL-terminated string allocated by the wrapper and
    // remains valid until the raw result is deleted.
    Some(
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned(),
    )
}
