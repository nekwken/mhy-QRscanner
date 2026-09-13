//! Aigis (Geetest v4) challenge handling for the passport auth flows.
//!
//! Contract confirmed against the official Android SDK behaviour and live
//! client traffic (2026-09-11):
//!
//! 1. `createLoginCaptcha` with an empty `x-rpc-aigis` header is refused with
//!    `retcode=-3101`; the challenge travels in the **response header**
//!    `x-rpc-aigis` (the body is empty).
//! 2. The challenge JSON carries the server-issued `session_id` and the
//!    Geetest config (`gt` = captcha id, `risk_type`).
//! 3. The client runs Geetest v4 with `riskType` and
//!    `userInfo={"session_id":…}`, then retries with
//!    `x-rpc-aigis: "<session_id>;<base64(result json)>"` where the result
//!    json is the Geetest validate payload enriched with `captcha_id` and
//!    `userInfo`.
//!
//! The solve itself is deliberately **human-in-the-loop**: this module serves
//! Geetest's official web widget on a localhost page, the user completes it in
//! their browser (it often passes invisibly, as it does in the official app),
//! and the page hands the result back. Nothing here bypasses the captcha.

use base64::Engine;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Mutex;
use std::time::Duration;

/// One server-issued aigis challenge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AigisChallenge {
    pub session_id: String,
    /// Geetest v4 captcha id (`gt`).
    pub gt: String,
    pub risk_type: String,
    pub mmt_type: Option<i64>,
}

impl AigisChallenge {
    /// Parse the raw `x-rpc-aigis` response header of a `-3101` refusal.
    pub fn parse(raw: &str) -> Option<Self> {
        let v: Value = serde_json::from_str(raw).ok()?;
        let session_id = v.get("session_id")?.as_str()?.to_string();
        let data = v.get("data")?;
        // `data` may arrive as an escaped JSON string or as a nested object.
        let data: Value = match data {
            Value::String(s) => serde_json::from_str(s).ok()?,
            d @ Value::Object(_) => d.clone(),
            _ => return None,
        };
        let gt = data.get("gt")?.as_str()?.to_string();
        let risk_type = data
            .get("risk_type")
            .and_then(|r| r.as_str())
            .unwrap_or("icon")
            .to_string();
        let mmt_type = v.get("mmt_type").and_then(|m| m.as_i64());
        Some(Self {
            session_id,
            gt,
            risk_type,
            mmt_type,
        })
    }

    /// Build the retry request's `x-rpc-aigis` header value from the user's
    /// Geetest result (`lot_number`/`captcha_output`/`pass_token`/`gen_time`):
    /// `"<session_id>;<base64(enriched json)>"`, byte-compatible with what the
    /// official SDK sends on the retry attempt.
    pub fn build_response_header(&self, geetest_result_json: &str) -> Result<String, String> {
        let mut result: Value =
            serde_json::from_str(geetest_result_json).map_err(|e| format!("bad result: {e}"))?;
        let obj = result.as_object_mut().ok_or("result must be an object")?;
        obj.insert("captcha_id".into(), json!(self.gt));
        obj.insert("userInfo".into(), json!({"session_id": self.session_id}));
        let b64 = base64::engine::general_purpose::STANDARD.encode(result.to_string());
        Ok(format!("{};{}", self.session_id, b64))
    }
}

/// A started-but-not-yet-completed interactive solve.
pub struct PendingSolve {
    pub challenge: AigisChallenge,
    rx: std::sync::mpsc::Receiver<String>,
}

static NEXT_SOLVE_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

fn pending() -> &'static Mutex<HashMap<u32, PendingSolve>> {
    static P: std::sync::OnceLock<Mutex<HashMap<u32, PendingSolve>>> = std::sync::OnceLock::new();
    P.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Start the solve server for this challenge WITHOUT opening a browser and
/// WITHOUT blocking: returns the solve-page URL (embed it in an in-app
/// webview or open it however the host wants) and a handle id for
/// [`take_result`]/[`cancel`].
pub fn begin_interactive(challenge: &AigisChallenge) -> Result<(u32, String), String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let url = format!("http://127.0.0.1:{port}/");
    let page = solver_page(challenge);
    let (tx, rx) = std::sync::mpsc::channel();
    let id = NEXT_SOLVE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut stream = stream;
            // Read the FULL request: headers and body can arrive in separate
            // TCP segments, and a single read truncated the JSON body into an
            // unparseable empty string.
            let Some(request) = read_http_request(&mut stream) else {
                continue;
            };
            let path = request
                .split_whitespace()
                .nth(1)
                .unwrap_or_default()
                .trim_end_matches('/')
                .to_string();
            if request.starts_with("GET") && path.ends_with("/finished") {
                let done_page = "<!doctype html><meta charset='utf-8'><body style=\"font-family:system-ui;display:flex;align-items:center;justify-content:center;height:100vh;margin:0\"><p>验证完成，请关闭此页面。</p></body>";
                let _ = stream.write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
                        done_page.len()
                    )
                    .as_bytes(),
                );
                let _ = stream.write_all(done_page.as_bytes());
                continue;
            }
            if request.starts_with("POST") && path.ends_with("/done") {
                let body = request
                    .split_once("\r\n\r\n")
                    .map(|(_, b)| b.trim().to_string())
                    .unwrap_or_default();
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nConnection: close\r\n\r\nok - you can close this tab",
                );
                let _ = tx.send(body);
                break;
            }
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
                    page.len()
                )
                .as_bytes(),
            );
            let _ = stream.write_all(page.as_bytes());
        }
    });

    pending().lock().unwrap().insert(
        id,
        PendingSolve {
            challenge: challenge.clone(),
            rx,
        },
    );
    Ok((id, url))
}

/// Wait for the solve result of a pending session and build the retry header
/// value. Consumes the pending entry.
pub fn take_result(id: u32, timeout: Duration) -> Result<String, String> {
    let pending = pending().lock().unwrap().remove(&id);
    let Some(pending) = pending else {
        return Err("aigis solve session not found (already taken or cancelled)".into());
    };
    let result = pending
        .rx
        .recv_timeout(timeout)
        .map_err(|_| "aigis solve timed out".to_string())?;
    pending.challenge.build_response_header(&result)
}

/// Drop a pending solve (user closed the dialog). The server thread exits on
/// its next send.
pub fn cancel(id: u32) {
    pending().lock().unwrap().remove(&id);
}

/// Blocking convenience for CLI flows: begin + open browser + wait.
pub fn solve_interactive(challenge: &AigisChallenge, timeout: Duration) -> Result<String, String> {
    let (id, url) = begin_interactive(challenge)?;
    open_in_browser(&url);
    println!("请在打开的浏览器页面完成图形验证（若无界面请访问）：{url}");
    take_result(id, timeout)
}

/// Read one complete HTTP request: loop until the header terminator, parse
/// `Content-Length`, then keep reading until the body is complete.
fn read_http_request(stream: &mut std::net::TcpStream) -> Option<String> {
    stream
        .set_read_timeout(Some(Duration::from_millis(1500)))
        .ok()?;
    let mut buf: Vec<u8> = Vec::with_capacity(8192);
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        match stream.read(&mut chunk) {
            Ok(0) => return None,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    break pos + 4;
                }
                if buf.len() > 64 * 1024 {
                    return None;
                }
            }
            Err(_) => return None,
        }
    };
    let headers = String::from_utf8_lossy(&buf[..header_end]).to_ascii_lowercase();
    let content_length = headers
        .lines()
        .find_map(|l| l.strip_prefix("content-length:"))
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(0);
    while buf.len() < header_end + content_length {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(_) => break,
        }
    }
    Some(String::from_utf8_lossy(&buf).into_owned())
}

fn solver_page(challenge: &AigisChallenge) -> String {
    let gt = js_escape(&challenge.gt);
    let risk_type = js_escape(&challenge.risk_type);
    let session_id = js_escape(&challenge.session_id);
    // Note: every literal `{`/`}` in the JS below is doubled for `format!`.
    format!(
        r#"<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<title>图形验证 — 完成后请回到工具</title>
<style>body{{font-family:system-ui,sans-serif;background:#fafafa;display:flex;flex-direction:column;align-items:center;justify-content:center;height:100vh;margin:0}}
#box{{padding:24px;border-radius:12px;background:#fff;box-shadow:0 2px 12px rgba(0,0,0,.08)}}#st{{margin-top:16px;color:#555}}</style>
</head>
<body>
<div id="box"><h3>图形验证</h3><div id="captcha"></div><p id="st">正在加载极验脚本…</p></div>
<script src="https://static.geetest.com/v4/gt4.js"></script>
<script>
var st = function (t) {{
  document.getElementById('st').textContent = t;
}};
window.onerror = function (m) {{
  st('脚本错误：' + m);
}};
window.onload = function () {{
  if (typeof initGeetest4 !== 'function') {{
    st('极验脚本加载失败（检查网络/代理后刷新本页重试）');
    return;
  }}
  st('极验脚本已加载，初始化…');
  // Both key spellings: current gt4.js builds read config.captchaId
  // (camelCase) while the official docs use captcha_id. Passing both is
  // harmless to either dialect; a missing captcha id makes the load fail and
  // gt4.js silently degrade into its bypass fallback (an object without
  // verify), which is the "page shows nothing" bug.
  initGeetest4({{
    captcha_id: '{gt}',
    captchaId: '{gt}',
    product: 'bind',
    riskType: '{risk_type}',
    userInfo: JSON.stringify({{session_id: '{session_id}'}})
  }}, function (captcha) {{
    st('极验已初始化，等待就绪…');
    captcha.onReady(function () {{
      // bind-mode trigger differs across gt4.js builds: current ones expose
      // showCaptcha(), older docs say verify(). Use whichever exists.
      var trigger = captcha.showCaptcha || captcha.verify;
      if (typeof trigger === 'function') {{
        st('开始验证…（智能验证通常无界面，若弹出点选请手动完成）');
        trigger.call(captcha);
      }} else {{
        st('等待校验结果…');
      }}
    }});
    captcha.onSuccess(function (result) {{
      st('验证成功，正在返回工具…');
      fetch('/done', {{method: 'POST', headers: {{'Content-Type': 'application/json'}}, body: JSON.stringify(result)}})
        .finally(function () {{ location.replace('/finished'); }});
    }});
    captcha.onError(function (e) {{
      st('验证出错：' + JSON.stringify(e));
    }});
    captcha.onClose(function () {{
      st('验证被关闭，请刷新本页重试。');
    }});
    // Backstop: a degraded/bypassed environment never fires onSuccess but may
    // still produce the validate payload; poll and hand it back if so.
    var polls = 0;
    var iv = setInterval(function () {{
      polls++;
      var v = null;
      try {{ v = captcha.getValidate ? captcha.getValidate() : null; }} catch (e) {{}}
      if (v) {{
        clearInterval(iv);
        st('已取得校验结果，正在返回工具…');
        fetch('/done', {{method: 'POST', headers: {{'Content-Type': 'application/json'}}, body: JSON.stringify(v)}})
          .finally(function () {{ location.replace('/finished'); }});
      }} else if (polls >= 60) {{
        clearInterval(iv);
      }}
    }}, 2000);
  }});
}};
</script>
</body>
</html>
"#
    )
}

fn js_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

fn open_in_browser(url: &str) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "", url])
            .spawn();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verbatim challenge header from the captured traffic (2026-09-11 23:26).
    const SAMPLE: &str = r#"{"session_id":"ba48e2bf04b64c3e8d83bddc6da6ae5b","mmt_type":1,"data":"{\"success\":1,\"gt\":\"caf244bd21555cc6ce52ceca524340b8\",\"new_captcha\":1,\"use_v4\":true,\"risk_type\":\"icon\"}"}"#;

    #[test]
    fn parses_the_captured_challenge_header() {
        let c = AigisChallenge::parse(SAMPLE).expect("must parse");
        assert_eq!(c.session_id, "ba48e2bf04b64c3e8d83bddc6da6ae5b");
        assert_eq!(c.gt, "caf244bd21555cc6ce52ceca524340b8");
        assert_eq!(c.risk_type, "icon");
        assert_eq!(c.mmt_type, Some(1));
    }

    #[test]
    fn parse_survives_nested_data_object_and_garbage() {
        let nested = r#"{"session_id":"s1","data":{"gt":"g1","risk_type":"slide"}}"#;
        let c = AigisChallenge::parse(nested).expect("nested must parse");
        assert_eq!(c.gt, "g1");
        assert_eq!(c.risk_type, "slide");
        assert!(AigisChallenge::parse("not json").is_none());
        assert!(AigisChallenge::parse(r#"{"data":"{}"}"#).is_none());
    }

    #[test]
    fn response_header_matches_the_captured_shape() {
        let c = AigisChallenge::parse(SAMPLE).unwrap();
        let result = r#"{"lot_number":"fa4c3c32","captcha_output":"NsiE","pass_token":"5c2358e7","gen_time":"1789140386"}"#;
        let header = c.build_response_header(result).unwrap();
        let (sid, b64) = header.split_once(';').expect("semicolon separator");
        assert_eq!(sid, c.session_id);
        use base64::Engine as _;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap();
        let v: Value = serde_json::from_slice(&decoded).unwrap();
        assert_eq!(v["captcha_id"], c.gt);
        assert_eq!(v["lot_number"], "fa4c3c32");
        assert_eq!(v["userInfo"]["session_id"], c.session_id);
    }

    #[test]
    fn solver_page_embeds_the_challenge_parameters() {
        let c = AigisChallenge::parse(SAMPLE).unwrap();
        let page = solver_page(&c);
        assert!(page.contains(c.gt.as_str()));
        assert!(page.contains(c.session_id.as_str()));
        assert!(page.contains("initGeetest4"));
    }
}
