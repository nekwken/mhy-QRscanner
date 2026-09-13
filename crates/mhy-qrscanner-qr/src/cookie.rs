//! Cookie builders for passport QR (verified 2026-09-11).

/// Device/BBS mid used in Cookie for scanQRLogin.
pub const DEFAULT_COOKIE_MID: &str = "0cdapswfd1_mhy";

/// App Cookie: `stoken={token};mid={cookie_mid}`.
///
/// `cookie_mid` is the **device/BBS mid** (`0cdapswfd1_mhy`), not passport `user_info.mid`.
pub fn passport_qr_cookie(stoken: &str, cookie_mid: &str) -> String {
    let mid = if cookie_mid.is_empty() {
        DEFAULT_COOKIE_MID
    } else {
        cookie_mid
    };
    format!("stoken={stoken};mid={mid}")
}
