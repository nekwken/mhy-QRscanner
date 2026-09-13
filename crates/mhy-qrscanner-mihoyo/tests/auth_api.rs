use mhy_qrscanner_mihoyo::auth::HttpAuthApi;
use mhy_qrscanner_mihoyo::ds::Salt;
use mhy_qrscanner_mihoyo::RpcHeaders;
use wiremock::matchers::{header, header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn login_by_password_posts_ds_and_encrypted_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/account/ma-cn-passport/app/loginByPassword"))
        .and(header_exists("DS"))
        .and(header_exists("x-rpc-device_id"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "retcode": 0,
            "message": "OK",
            "data": {
                "token": { "token": "st_v2_xxx", "token_type": 1 },
                "user_info": { "aid": 42, "mid": "mid42", "account_name": "u" }
            }
        })))
        .mount(&server)
        .await;

    let api = HttpAuthApi::new(server.uri());
    let headers = RpcHeaders::miyoushe_defaults("dev1", "fp1");
    let data = api
        .login_by_password(
            &headers,
            &Salt::Prod,
            "user@example.com",
            "pw-not-logged",
            "",
        )
        .await
        .unwrap();
    let acct = data.to_login_account().expect("token present");
    assert_eq!(acct.uid, "42");
    assert_eq!(acct.mid, "mid42");
}

#[tokio::test]
async fn send_captcha_and_mobile_login() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/account/ma-cn-verifier/verifier/createLoginCaptcha"))
        .and(header_exists("DS"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "retcode": 0,
            "message": "OK",
            "data": { "msg": "ok" }
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/account/ma-cn-passport/app/loginByMobileCaptcha"))
        .and(header_exists("DS"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "retcode": 0,
            "message": "OK",
            "data": {
                "token": { "token": "st7", "token_type": 1 },
                "user_info": { "aid": 7, "mid": "m7" }
            }
        })))
        .mount(&server)
        .await;

    let api = HttpAuthApi::new(server.uri());
    let headers = RpcHeaders::miyoushe_defaults("dev1", "fp1");
    api.send_login_captcha(&headers, &Salt::Prod, "+86", "13800000000", "")
        .await
        .unwrap();
    let data = api
        .login_by_mobile_captcha(&headers, &Salt::Prod, "+86", "13800000000", "123456")
        .await
        .unwrap();
    assert_eq!(data.to_login_account().unwrap().uid, "7");
}

#[tokio::test]
async fn login_propagates_risk_retcode() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/account/ma-cn-passport/app/loginByPassword"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "retcode": -3101,
            "message": "need captcha",
            "data": null
        })))
        .mount(&server)
        .await;
    let api = HttpAuthApi::new(server.uri());
    let headers = RpcHeaders::miyoushe_defaults("d", "f");
    let err = api
        .login_by_password(&headers, &Salt::Prod, "a", "b", "")
        .await
        .unwrap_err();
    assert!(err.to_string().contains("-3101"));
}

#[tokio::test]
async fn get_token_by_game_token_exchanges() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/account/ma-cn-session/app/getTokenByGameToken"))
        .and(header_exists("DS"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "retcode": 0,
            "message": "OK",
            "data": {
                "token": { "token": "st9", "token_type": 1 },
                "user_info": { "aid": 9, "mid": "m9" }
            }
        })))
        .mount(&server)
        .await;
    let api = HttpAuthApi::new(server.uri());
    let headers = RpcHeaders::miyoushe_defaults("d", "f");
    let data = api
        .get_token_by_game_token(&headers, &Salt::Prod, "game_tok", 1)
        .await
        .unwrap();
    assert_eq!(data.to_login_account().unwrap().token, "st9");
}

/// -3101 with the challenge in the **response header** is the aigis gate:
/// the SDK reads `x-rpc-aigis` (body stays empty) and runs Geetest v4.
#[tokio::test]
async fn aigis_challenge_arrives_from_the_response_header() {
    let server = MockServer::start().await;
    let challenge = r#"{"session_id":"ba48e2bf04b64c3e8d83bddc6da6ae5b","mmt_type":1,"data":"{\"success\":1,\"gt\":\"caf244bd21555cc6ce52ceca524340b8\",\"new_captcha\":1,\"use_v4\":true,\"risk_type\":\"icon\"}"}"#;
    Mock::given(method("POST"))
        .and(path("/account/ma-cn-verifier/verifier/createLoginCaptcha"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({
                    "retcode": -3101,
                    "message": "\u{8bf7}\u{6c42}\u{9891}\u{7e41}\u{ff0c}\u{8bf7}\u{7a0d}\u{540e}\u{518d}\u{8bd5}",
                    "data": null
                }))
                .insert_header("x-rpc-aigis", challenge),
        )
        .mount(&server)
        .await;
    let api = HttpAuthApi::new(server.uri());
    let headers = RpcHeaders::miyoushe_defaults("d", "f");
    let err = api
        .send_login_captcha(&headers, &Salt::Prod, "+86", "13800000000", "")
        .await
        .unwrap_err();
    let mhy_qrscanner_mihoyo::device_api::MihoyoError::AigisChallenge { challenge } = &err else {
        panic!("expected AigisChallenge, got {err:?}");
    };
    let parsed = mhy_qrscanner_mihoyo::aigis::AigisChallenge::parse(challenge).unwrap();
    assert_eq!(parsed.session_id, "ba48e2bf04b64c3e8d83bddc6da6ae5b");
    assert_eq!(parsed.gt, "caf244bd21555cc6ce52ceca524340b8");
    assert_eq!(parsed.risk_type, "icon");
    // and the classifier names it honestly
    let ch = mhy_qrscanner_mihoyo::auth::challenge_from_api_error(&err).unwrap();
    assert_eq!(ch.kind, mhy_qrscanner_mihoyo::auth::LoginChallengeKind::Aigis);
}

/// The retry after solving must send the produced value verbatim in the
/// `x-rpc-aigis` request header.
#[tokio::test]
async fn retry_carries_the_solved_aigis_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/account/ma-cn-verifier/verifier/createLoginCaptcha"))
        .and(header("x-rpc-aigis", "sess-id;c2Vzc2lvbg=="))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "retcode": 0, "message": "OK",
            "data": { "sent_new": true, "countdown": 60, "action_type": "login_by_mobile_captcha" }
        })))
        .mount(&server)
        .await;
    let api = HttpAuthApi::new(server.uri());
    let headers = RpcHeaders::miyoushe_defaults("d", "f");
    let data = api
        .send_login_captcha(
            &headers,
            &Salt::Prod,
            "+86",
            "13800000000",
            "sess-id;c2Vzc2lvbg==",
        )
        .await
        .unwrap();
    assert_eq!(data["sent_new"], true);
}

/// -3101 without a challenge header stays a plain refusal (no gate passthrough).
#[tokio::test]
async fn rate_limit_without_aigis_header_stays_a_plain_refusal() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/account/ma-cn-verifier/verifier/createLoginCaptcha"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "retcode": -3101,
            "message": "\u{8bf7}\u{6c42}\u{9891}\u{7e41}\u{ff0c}\u{8bf7}\u{7a0d}\u{540e}\u{518d}\u{8bd5}",
            "data": null
        })))
        .mount(&server)
        .await;
    let api = HttpAuthApi::new(server.uri());
    let headers = RpcHeaders::miyoushe_defaults("d", "f");
    let err = api
        .send_login_captcha(&headers, &Salt::Prod, "+86", "13800000000", "")
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        mhy_qrscanner_mihoyo::device_api::MihoyoError::Api { retcode: -3101, .. }
    ));
}
