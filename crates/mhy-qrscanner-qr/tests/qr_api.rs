use mhy_qrscanner_mihoyo::ds::Salt;
use mhy_qrscanner_qr::{
    panda_scan_from_v1_url, passport_qr_cookie, HttpQrApi, PassportQrConfirmRequest,
    PassportQrScanRequest, RpcHeaders,
};
use wiremock::matchers::{header, method, path};
use wiremock::{Match, Mock, MockServer, Request as WRequest, ResponseTemplate};

struct AnyHeader(&'static str);

impl Match for AnyHeader {
    fn matches(&self, request: &WRequest) -> bool {
        request
            .headers
            .get(self.0)
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    }
}

#[tokio::test]
async fn panda_scan_uses_hk4e_path_and_numeric_app_id() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/hk4e_cn/combo/panda/qrcode/scan"))
        .and(header("x-rpc-device_id", "3be54a3bcb4a7a9a"))
        .and(header("x-rpc-app_id", "bll8iq97cem8"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "retcode": 0,
            "message": "OK",
            "data": {
                "passport_qr_url": "https://user.mihoyo.com/login-platform/mobile.html?tk=x#/login/qr"
            }
        })))
        .mount(&server)
        .await;

    let api = HttpQrApi::new(server.uri(), "https://unused.example");
    let headers = RpcHeaders::miyoushe_defaults("3be54a3bcb4a7a9a", "38d7fbdb1d6a2");
    assert_eq!(headers.app_id, "bll8iq97cem8");
    let req = panda_scan_from_v1_url(
        "https://user.mihoyo.com/qr_code_in_game.html?app_id=4&ticket=6aa2dd1009ba9d09801efd96&biz_key=hk4e_cn",
        "3be54a3bcb4a7a9a",
        "bll8iq97cem8",
        1789058335,
    )
    .unwrap();
    assert_eq!(req.app_id, 4);
    let data = api.panda_scan(&headers, "hk4e_cn", &req).await.unwrap();
    assert!(data.passport_qr_url.contains("tk=x"));
}

#[tokio::test]
async fn passport_scan_sends_ds_and_device_mid_cookie() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/account/ma-cn-passport/app/scanQRLogin"))
        .and(AnyHeader("DS"))
        .and(header(
            "Cookie",
            "stoken=v2_c83nEXAMPLECAE=;mid=0cdapswfd1_mhy",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "retcode": 0,
            "message": "OK",
            "data": {
                "app_id": "c76ync6mutq8",
                "app_name": "原神",
                "client_type": 3,
                "account_disp_name": "195******50",
                "risk_note": ""
            }
        })))
        .mount(&server)
        .await;

    let api = HttpQrApi::new("https://unused.example", server.uri());
    let headers = RpcHeaders::miyoushe_defaults("3be54a3bcb4a7a9a", "38d7fbdb1d6a2");
    let req = PassportQrScanRequest {
        ticket: "314efe23-26fe-4d86-8b19-30efe2e3a48c".into(),
        token_types: vec!["1".into()],
    };
    let data = api
        .passport_scan(
            &headers,
            &Salt::Prod,
            &passport_qr_cookie("v2_c83nEXAMPLECAE=", "0cdapswfd1_mhy"),
            &req,
        )
        .await
        .unwrap();
    assert_eq!(data.app_name, "原神");
    assert_eq!(data.client_type, Some(3));
}

#[tokio::test]
async fn passport_confirm_returns_data() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/account/ma-cn-passport/app/confirmQRLogin"))
        .and(AnyHeader("DS"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "retcode": 0,
            "message": "OK",
            "data": { "ok": true }
        })))
        .mount(&server)
        .await;

    let api = HttpQrApi::new("https://unused.example", server.uri());
    let headers = RpcHeaders::miyoushe_defaults("dev1", "fp1");
    let req = PassportQrConfirmRequest {
        ticket: "tk".into(),
        token_types: vec!["1".into()],
        confirm: None,
    };
    let data = api
        .passport_confirm(&headers, &Salt::Prod, "stoken=st1;mid=0cdapswfd1_mhy", &req)
        .await
        .unwrap();
    assert_eq!(data["ok"], true);
}
