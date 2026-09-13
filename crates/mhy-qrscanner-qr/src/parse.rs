//! QR URL parsers and request builders.

use crate::types::PandaQrScanRequest;
use mhy_qrscanner_mihoyo::device_api::MihoyoError;

/// Heuristic: V1 game QR URL (`.../qr_code_in_game.html?app_id=&ticket=`).
pub fn is_v1_qr_url(url: &str) -> bool {
    parse_v1_qr_url(url).is_some()
}

/// Extract just the ticket from a V1 game QR URL.
pub fn ticket_from_v1_url(url: &str) -> Option<String> {
    parse_v1_qr_url(url).map(|(_, ticket, _)| ticket)
}

/// Parse V1 QR URL into (app_id, ticket, biz_key).
pub fn parse_v1_qr_url(url: &str) -> Option<(String, String, String)> {
    let u = url::Url::parse(url).ok()?;
    let host = u.host_str().unwrap_or_default();
    if !(host.ends_with("mihoyo.com") || host.ends_with("mihayo.com")) {
        return None;
    }
    if !u.path().ends_with("qr_code_in_game.html") {
        return None;
    }
    let mut app_id = None;
    let mut ticket = None;
    let mut biz_key = String::new();
    for (k, v) in u.query_pairs() {
        match k.as_ref() {
            "app_id" if !v.is_empty() => app_id = Some(v.to_string()),
            "ticket" if !v.is_empty() => ticket = Some(v.to_string()),
            "biz_key" => biz_key = v.to_string(),
            _ => {}
        }
    }
    Some((app_id?, ticket?, biz_key))
}

/// Parse V2 QR URL ticket from `?tk=` (query and/or fragment).
pub fn parse_v2_qr_url(url: &str) -> Option<(String, Vec<String>)> {
    let u = url::Url::parse(url).ok()?;
    let host = u.host_str().unwrap_or_default();
    if !host.ends_with("mihoyo.com") && !host.ends_with("mihayo.com") {
        return None;
    }
    let mut search = u.query().unwrap_or("").to_string();
    if let Some(frag) = u.fragment() {
        if let Some((_, q)) = frag.split_once('?') {
            if !search.is_empty() {
                search.push('&');
            }
            search.push_str(q);
        }
    }
    let mut tk = None;
    let mut token_types = Vec::new();
    for (k, v) in url::form_urlencoded::parse(search.as_bytes()) {
        match k.as_ref() {
            "tk" if !v.is_empty() => tk = Some(v.to_string()),
            "token_types" => token_types.push(v.to_string()),
            _ => {}
        }
    }
    Some((tk?, token_types))
}

/// Build a panda scan request from a V1 QR URL + device identity.
///
/// `passport_app_id` is Miyoushe App id (`bll8iq97cem8` for 2.113.1).
pub fn panda_scan_from_v1_url(
    url: &str,
    device_id: &str,
    passport_app_id: &str,
    ts: u64,
) -> Result<PandaQrScanRequest, MihoyoError> {
    let (app_id_str, ticket, _) = parse_v1_qr_url(url)
        .ok_or_else(|| MihoyoError::InvalidResponse("not a v1 qr url".into()))?;
    let app_id = app_id_str
        .parse::<i64>()
        .map_err(|_| MihoyoError::InvalidResponse(format!("bad app_id {app_id_str}")))?;
    Ok(PandaQrScanRequest {
        passport_app_id: passport_app_id.to_string(),
        ticket,
        app_id,
        device: device_id.to_string(),
        ts,
    })
}

/// Default panda host for Genshin CN (`hk4e-sdk.mihoyo.com`).
pub const HK4E_PANDA_BASE: &str = "https://hk4e-sdk.mihoyo.com";
/// Default passport host.
pub const PASSPORT_BASE: &str = "https://passport-api.mihoyo.com";

/// Derive the panda (game SDK) host for a `game_biz`.
///
/// CN pattern observed for Genshin: `https://<prefix>-sdk.mihoyo.com` where
/// `<prefix>` is the part before `_` (`hk4e_cn` → `hk4e`). The same shape is
/// assumed for other CN titles and is **inferred, not verified**; callers can
/// always override with `--panda-host`.
pub fn panda_host_for(game_biz: &str) -> String {
    let prefix = game_biz.split('_').next().unwrap_or(game_biz);
    format!("https://{prefix}-sdk.mihoyo.com")
}

/// True when the panda host is a derived guess rather than a verified value.
pub fn panda_host_is_derived(game_biz: &str) -> bool {
    panda_host_for(game_biz) != HK4E_PANDA_BASE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_url_detection() {
        let url =
            "https://user.mihoyo.com/qr_code_in_game.html?app_id=4&ticket=tk1&biz_key=hk4e_cn";
        assert!(is_v1_qr_url(url));
        let (app, tk, biz) = parse_v1_qr_url(url).unwrap();
        assert_eq!(app, "4");
        assert_eq!(tk, "tk1");
        assert_eq!(biz, "hk4e_cn");
        assert!(!is_v1_qr_url(
            "https://user.mihoyo.com/mobile.html#/login/qr?tk=x"
        ));
    }

    #[test]
    fn v2_url_parse_query_and_fragment() {
        let url = "https://user.mihoyo.com/login-platform/mobile.html?expire=1&tk=abc&token_types=1#/login/qr";
        let (tk, types) = parse_v2_qr_url(url).unwrap();
        assert_eq!(tk, "abc");
        assert_eq!(types, vec!["1".to_string()]);
    }

    #[test]
    fn panda_host_derivation() {
        assert_eq!(panda_host_for("hk4e_cn"), HK4E_PANDA_BASE);
        assert!(!panda_host_is_derived("hk4e_cn"));
        assert_eq!(panda_host_for("hkrpg_cn"), "https://hkrpg-sdk.mihoyo.com");
        assert_eq!(panda_host_for("nap_cn"), "https://nap-sdk.mihoyo.com");
        assert_eq!(panda_host_for("bh3_cn"), "https://bh3-sdk.mihoyo.com");
        // non-Genshin CN hosts are derived guesses
        assert!(panda_host_is_derived("hkrpg_cn"));
    }

    #[test]
    fn panda_from_v1_url() {
        let req = panda_scan_from_v1_url(
            "https://user.mihoyo.com/qr_code_in_game.html?app_id=4&ticket=6aa2dd1009ba9d09801efd96&biz_key=hk4e_cn",
            "3be54a3bcb4a7a9a",
            "bll8iq97cem8",
            1789058335,
        )
        .unwrap();
        assert_eq!(req.app_id, 4);
        assert_eq!(req.ticket, "6aa2dd1009ba9d09801efd96");
        assert_eq!(req.passport_app_id, "bll8iq97cem8");
        assert_eq!(req.device, "3be54a3bcb4a7a9a");
    }
}
