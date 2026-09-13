//! mhy-qrscanner-live: Bilibili live-stream capture for the QR race.
//!
//! The charter input path: pull a room's live stream directly (lower latency
//! and no player window needed) and pump decoded gray frames into the QR
//! pipeline. Stream resolution uses the public `getRoomPlayInfo` endpoint;
//! frame extraction shells out to a local `ffmpeg` (flv → raw gray), which
//! keeps this crate free of demuxer/decoder code.

use serde_json::Value;
use std::collections::VecDeque;
use std::net::{TcpStream, ToSocketAddrs};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 一个监控源的实时流统计（GUI 轮询展示用）。
#[derive(Default, Debug, Clone, PartialEq)]
pub struct StreamStats {
    pub frames: u64,
    pub width: u32,
    pub height: u32,
    /// 实测解码帧率（滚动窗口，帧/s）。
    pub fps: f64,
    /// 与流服务器的 TCP 连接时延（≈ping）；None = 尚未测量。
    pub ping_ms: Option<u64>,
    /// 解出过二维码的帧数（同一张码反复解出会累计）。
    pub decoded: u64,
    /// 最近一帧的解码结果说明（供界面显示「在扫什么」）。
    pub hint: String,
}

pub type SharedStats = Arc<Mutex<StreamStats>>;

/// 测量到流服务器的 TCP 连接时延（≈ping）。从流 URL 解析 scheme/host/port。
pub fn ping_stream_server(url: &str, timeout: Duration) -> Result<u64, String> {
    // 形如 https://d1--cn-gotcha09.bilivideo.com/live-bvc/... 或 http://host:port/...
    let (scheme, rest) = url
        .split_once("://")
        .ok_or_else(|| "url missing scheme".to_string())?;
    let authority = rest.split('/').next().unwrap_or_default();
    let (host, port) = match scheme {
        "https" => {
            let (h, p) = authority.split_once(':').unwrap_or((authority, "443"));
            (h.to_string(), p.parse::<u16>().unwrap_or(443))
        }
        _ => {
            let (h, p) = authority.split_once(':').unwrap_or((authority, "80"));
            (h.to_string(), p.parse::<u16>().unwrap_or(80))
        }
    };
    // 域名不是 IP 字面量——必须走 DNS 解析；CDN 常解析出多个地址，逐个测连。
    let addrs: Vec<_> = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|e| format!("resolve {host}: {e}"))?
        .collect();
    if addrs.is_empty() {
        return Err(format!("resolve {host}: no addresses"));
    }
    let start = Instant::now();
    let mut last_err: Option<String> = None;
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(_) => return Ok(start.elapsed().as_millis() as u64),
            Err(e) => last_err = Some(format!("connect {addr}: {e}")),
        }
    }
    Err(last_err.unwrap_or_else(|| "connect failed".into()))
}

#[derive(Debug, thiserror::Error)]
pub enum LiveError {
    #[error("api error code={code} message={message}")]
    Api { code: i64, message: String },
    #[error("no usable stream in playurl response")]
    NoStream,
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("ffmpeg error: {0}")]
    Ffmpeg(String),
}

/// A resolved playable live stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveStream {
    pub room_id: u64,
    /// Fully-qualified stream URL (base_url + host + extra).
    pub url: String,
    pub format: String,
}

/// Resolve a Bilibili room into a playable flv stream URL.
///
/// `api_host` is injectable for tests. Uses the public endpoint (no cookies);
/// quality is whatever the anonymous tier grants — QR codes survive that.
pub fn resolve_play_url(
    room_id: &str,
    api_host: &str,
    client: &reqwest::blocking::Client,
) -> Result<LiveStream, LiveError> {
    let url = format!(
        "{api_host}/xlive/web-room/v2/index/getRoomPlayInfo?room_id={room_id}\
         &protocol=0&format=0&codec=0&qn=10000&platform=web&ptype=16"
    );
    let text = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .header("Referer", "https://live.bilibili.com/")
        .timeout(Duration::from_secs(15))
        .send()
        .map_err(|e| LiveError::InvalidResponse(format!("{e}")))?
        .text()
        .map_err(|e| LiveError::InvalidResponse(format!("{e}")))?;
    parse_play_url(room_id, &text)
}

/// Parse the `getRoomPlayInfo` body: pick the first http_stream/flv codec and
/// assemble `base_url + url_info[0].host + url_info[0].extra`.
pub fn parse_play_url(room_id: &str, body: &str) -> Result<LiveStream, LiveError> {
    let v: Value =
        serde_json::from_str(body).map_err(|e| LiveError::InvalidResponse(format!("json: {e}")))?;
    if v["code"].as_i64().unwrap_or(-1) != 0 {
        return Err(LiveError::Api {
            code: v["code"].as_i64().unwrap_or(-1),
            message: v["message"].as_str().unwrap_or_default().to_string(),
        });
    }
    let streams = &v["data"]["playurl_info"]["playurl"]["stream"];
    let streams = streams
        .as_array()
        .ok_or_else(|| LiveError::InvalidResponse("stream is not a list".into()))?;

    for stream in streams {
        if stream["protocol_name"].as_str() != Some("http_stream") {
            continue;
        }
        let Some(formats) = stream["format"].as_array() else {
            continue;
        };
        for format in formats {
            let Some(codecs) = format["codec"].as_array() else {
                continue;
            };
            for codec in codecs {
                let base = codec["base_url"].as_str().unwrap_or_default();
                let host = codec["url_info"][0]["host"].as_str().unwrap_or_default();
                let extra = codec["url_info"][0]["extra"].as_str().unwrap_or_default();
                if base.is_empty() || host.is_empty() {
                    continue;
                }
                return Ok(LiveStream {
                    room_id: room_id.parse().unwrap_or(0),
                    url: format!("{host}{base}{extra}"),
                    format: format["format_name"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                });
            }
        }
    }
    Err(LiveError::NoStream)
}

/// One decoded gray frame from the pump.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrayFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// A running ffmpeg process that emits fixed-size raw gray frames on stdout.
///
/// Scale keeps the aspect ratio and pads to the exact frame size so raw reads
/// stay aligned; `fps` throttles decode work (QR hunting needs no 60 fps).
pub struct FramePump {
    child: Child,
    width: u32,
    height: u32,
    frame_bytes: usize,
    frames: u64,
    frame_times: VecDeque<Instant>,
}

impl FramePump {
    pub fn start(
        ffmpeg: &str,
        url: &str,
        width: u32,
        height: u32,
        fps: u32,
        referer: &str,
    ) -> Result<Self, LiveError> {
        let filter = format!(
            "fps={fps},scale={width}:{height}:force_original_aspect_ratio=decrease,\
             pad={width}:{height}:(ow-iw)/2:(oh-ih)/2:black"
        );
        let headers = format!("Referer: {referer}\r\nUser-Agent: Mozilla/5.0\r\n");
        let mut command = Command::new(ffmpeg);
        command
            .args([
                "-loglevel",
                "error",
                "-headers",
                &headers,
                "-i",
                url,
                "-vf",
                &filter,
                "-f",
                "rawvideo",
                "-pix_fmt",
                "gray",
                "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        // GUI 进程里每个 ffmpeg 子进程都会带一个控制台黑框，压掉它。
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let child = command
            .spawn()
            .map_err(|e| LiveError::Ffmpeg(format!("spawn: {e} (is ffmpeg on PATH?)")))?;
        Ok(Self {
            child,
            width,
            height,
            frame_bytes: (width * height) as usize,
            frames: 0,
            frame_times: VecDeque::new(),
        })
    }

    /// 已解码的完整帧数。
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// 实测帧率：滚动窗口内相邻帧间隔的均值倒数（帧/s）。
    pub fn fps(&self) -> f64 {
        if self.frame_times.len() < 2 {
            return 0.0;
        }
        let newest = self.frame_times.back().unwrap();
        let oldest = self.frame_times.front().unwrap();
        let span = newest.duration_since(*oldest).as_secs_f64();
        if span <= 0.0 {
            return 0.0;
        }
        (self.frame_times.len() - 1) as f64 / span
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Block until one full frame is available; `None` when the stream ended.
    pub fn read_frame(&mut self) -> Option<GrayFrame> {
        use std::io::Read;
        let mut data = vec![0u8; self.frame_bytes];
        let mut filled = 0;
        while filled < self.frame_bytes {
            let n = self.child.stdout.as_mut()?.read(&mut data[filled..]).ok()?;
            if n == 0 {
                return None;
            }
            filled += n;
        }
        self.frames += 1;
        self.frame_times.push_back(Instant::now());
        while self.frame_times.len() > 10 {
            self.frame_times.pop_front();
        }
        Some(GrayFrame {
            width: self.width,
            height: self.height,
            data,
        })
    }
}

impl Drop for FramePump {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "code": 0, "message": "0",
      "data": {
        "room_id": 741,
        "playurl_info": { "playurl": { "stream": [
          { "protocol_name": "http_hls", "format": [] },
          { "protocol_name": "http_stream", "format": [
            { "format_name": "flv", "codec": [
              { "codec_name": "avc", "base_url": "/live-bvc/live_1.flv?",
                "url_info": [ { "host": "https://host-a.example",
                                "extra": "expires=1&token=x" } ] }
            ] }
          ] }
        ] } }
      }
    }"#;

    #[test]
    fn parses_stream_url_from_playurl_body() {
        let s = parse_play_url("741", SAMPLE).unwrap();
        assert_eq!(s.room_id, 741);
        assert_eq!(s.format, "flv");
        assert_eq!(
            s.url,
            "https://host-a.example/live-bvc/live_1.flv?expires=1&token=x"
        );
    }

    #[test]
    fn api_error_surfaces_with_code_and_message() {
        let err =
            parse_play_url("1", r#"{"code": 19000009, "message": "直播间不存在"}"#).unwrap_err();
        assert!(matches!(err, LiveError::Api { code: 19000009, .. }));
    }

    #[test]
    fn missing_stream_reports_no_stream() {
        let err = parse_play_url(
            "1",
            r#"{"code":0,"data":{"playurl_info":{"playurl":{"stream":[{"protocol_name":"http_hls","format":[]}]}}}}"#,
        )
        .unwrap_err();
        assert!(matches!(err, LiveError::NoStream));
    }
}
