use crate::sys;
use std::ffi::{CStr, CString};

#[macro_export]
macro_rules! deref{
    // Base case: single identifier
    ($base:ident) => { 
        $base 
    };

    // Base case: single identifier
    ($a:expr, $b: ident) => { 
        *(*$a).$b
    };

    // Base case: single identifier
    ($a:ident, $b: ident) => { 
        *(*$a).$b
    };

    // Recursive case: dereference
    ($a:expr, $b:ident, $($rest:ident),+) => { 
        deref!((*$a).$b, $($rest),*)
    };

    // Recursive case: dereference
    ($a:ident, $b:ident, $($rest:ident),+) => { 
        deref!((*$a).$b, $($rest),*)
    };
}
pub trait ToU32Result {
    fn to_u32_result(self, err_str: &str) -> Result<u32, String>;
}

impl ToU32Result for i32 {
    fn to_u32_result(self, err_str: &str) -> Result<u32, String> {
        if self >= 0 {
            Ok(self as u32)
        } else {
            let mut description: [std::os::raw::c_char; sys::AV_ERROR_MAX_STRING_SIZE as usize] =
                [0; sys::AV_ERROR_MAX_STRING_SIZE as usize];
            let res: &CStr = unsafe {
                sys::av_strerror(
                    self,
                    description.as_mut_ptr(),
                    sys::AV_ERROR_MAX_STRING_SIZE as usize,
                );
                CStr::from_ptr(description.as_ptr())
            };
            Err(format!("{}. {}", res.to_string_lossy(), err_str))
        }
    }
}

pub fn to_cstring(str: &str) -> CString {
    CString::new(str).expect("could not create cstring")
}
