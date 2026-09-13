use std::io;
use std::ptr;
use std::slice;
use winapi::shared::minwindef::{BYTE, DWORD};
use winapi::um::dpapi::{CryptProtectData, CryptUnprotectData};
use winapi::um::winbase::LocalFree;
use winapi::um::wincrypt::DATA_BLOB;

fn input_blob(data: &[u8]) -> DATA_BLOB {
    DATA_BLOB {
        cbData: data.len() as DWORD,
        pbData: data.as_ptr() as *mut BYTE,
    }
}

unsafe fn output_vec(blob: DATA_BLOB) -> Vec<u8> {
    let result = slice::from_raw_parts(blob.pbData, blob.cbData as usize).to_vec();
    LocalFree(blob.pbData as *mut _);
    result
}

pub fn protect(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut input = input_blob(data);
    let mut output = DATA_BLOB {
        cbData: 0,
        pbData: ptr::null_mut(),
    };

    let ok = unsafe {
        CryptProtectData(
            &mut input,
            ptr::null(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            &mut output,
        )
    };

    if ok == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(unsafe { output_vec(output) })
}

pub fn unprotect(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut input = input_blob(data);
    let mut output = DATA_BLOB {
        cbData: 0,
        pbData: ptr::null_mut(),
    };

    let ok = unsafe {
        CryptUnprotectData(
            &mut input,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            &mut output,
        )
    };

    if ok == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(unsafe { output_vec(output) })
}
