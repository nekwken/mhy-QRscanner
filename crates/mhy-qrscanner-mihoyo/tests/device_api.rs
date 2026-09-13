use mhy_qrscanner_mihoyo::device_api::{DeviceApi, GetFpRequest, HttpDeviceApi};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn get_ext_list_reads_data() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/device-fp/api/getExtList"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "retcode": 0,
            "message": "OK",
            "data": ["model", "brand"]
        })))
        .mount(&server)
        .await;

    let api = HttpDeviceApi::new(server.uri());
    let data = api.get_ext_list("2", "bbs_cn").await.unwrap();
    assert_eq!(data, serde_json::json!(["model", "brand"]));
}

#[tokio::test]
async fn get_fp_reads_device_fingerprint() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/device-fp/api/getFp"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "retcode": 0,
            "message": "OK",
            "data": {
                "device_fp": "abcdef1234567"
            }
        })))
        .mount(&server)
        .await;

    let api = HttpDeviceApi::new(server.uri());
    let request = GetFpRequest {
        device_id: "0123456789abcdef".to_string(),
        seed_id: "00000000-0000-4000-8000-000000000000".to_string(),
        seed_time: "1700000000000".to_string(),
        platform: "2".to_string(),
        device_fp: "1111111111111".to_string(),
        app_name: "bbs_cn".to_string(),
        ext_fields: serde_json::json!({"model": "Xiaomi 14"}).to_string(),
        bbs_device_id: "00000000-0000-4000-8000-000000000000".to_string(),
    };

    let response = api.get_fp(&request).await.unwrap();
    assert_eq!(response.device_fp, "abcdef1234567");
}

#[tokio::test]
async fn get_fp_rejects_empty_fingerprint() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/device-fp/api/getFp"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "retcode": 0,
            "message": "OK",
            "data": { "device_fp": "" }
        })))
        .mount(&server)
        .await;

    let api = HttpDeviceApi::new(server.uri());
    let request = GetFpRequest {
        device_id: "0123456789abcdef".to_string(),
        seed_id: "00000000-0000-4000-8000-000000000000".to_string(),
        seed_time: "1700000000000".to_string(),
        platform: "2".to_string(),
        device_fp: "1111111111111".to_string(),
        app_name: "bbs_cn".to_string(),
        ext_fields: serde_json::json!({"model": "Xiaomi 14"}).to_string(),
        bbs_device_id: "00000000-0000-4000-8000-000000000000".to_string(),
    };

    let error = api.get_fp(&request).await.unwrap_err();
    assert!(
        matches!(
            error,
            mhy_qrscanner_mihoyo::device_api::MihoyoError::InvalidResponse(_)
        ),
        "expected InvalidResponse, got {error:?}"
    );
}

#[tokio::test]
async fn get_fp_rejects_non_200_inner_code() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/device-fp/api/getFp"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "retcode": 0,
            "message": "OK",
            "data": { "code": 500, "device_fp": "abcdef1234567" }
        })))
        .mount(&server)
        .await;

    let api = HttpDeviceApi::new(server.uri());
    let request = GetFpRequest {
        device_id: "0123456789abcdef".to_string(),
        seed_id: "00000000-0000-4000-8000-000000000000".to_string(),
        seed_time: "1700000000000".to_string(),
        platform: "2".to_string(),
        device_fp: "1111111111111".to_string(),
        app_name: "bbs_cn".to_string(),
        ext_fields: serde_json::json!({"model": "Xiaomi 14"}).to_string(),
        bbs_device_id: "00000000-0000-4000-8000-000000000000".to_string(),
    };

    let error = api.get_fp(&request).await.unwrap_err();
    assert!(
        matches!(
            error,
            mhy_qrscanner_mihoyo::device_api::MihoyoError::InvalidResponse(_)
        ),
        "expected InvalidResponse, got {error:?}"
    );
}
