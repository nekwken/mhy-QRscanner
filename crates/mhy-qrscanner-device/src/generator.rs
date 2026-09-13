use md5::{Digest, Md5};
use mhy_qrscanner_core::{AccountId, ClientProfile, DeviceProfile, GameBiz};
use rand::Rng;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Built-in device templates. Different brands look "stranger" to risk control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceTemplate {
    /// Original Xiaomi 14 / Android 14 (known-good A2 path).
    Xiaomi14,
    /// Samsung S24 Ultra / Android 14.
    SamsungS24,
    /// OPPO Find X7 / Android 14.
    OppoFindX7,
    /// vivo X100 / Android 14.
    VivoX100,
    /// Older Samsung A52 / Android 12 (more "dated").
    SamsungA52,
}

impl DeviceTemplate {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "xiaomi14" | "xiaomi" => Some(Self::Xiaomi14),
            "samsung-s24" | "s24" | "samsungs24" => Some(Self::SamsungS24),
            "oppo-find-x7" | "oppo" => Some(Self::OppoFindX7),
            "vivo-x100" | "vivo" => Some(Self::VivoX100),
            "samsung-a52" | "a52" => Some(Self::SamsungA52),
            _ => None,
        }
    }

    fn fields(&self) -> DeviceFields {
        match self {
            Self::Xiaomi14 => DeviceFields {
                model: "Xiaomi 14",
                brand: "Xiaomi",
                manufacturer: "Xiaomi",
                product: "houji",
                android: "14",
                sdk: "34",
                screen: "1200x2670",
                display: "UKQ1.231003.002 release-keys",
                device_info: "Xiaomi/houji/houji:14/UKQ1.231003.002/V816IR:user/release-keys",
                ram: "12288",
                rom: "512",
            },
            Self::SamsungS24 => DeviceFields {
                model: "SM-S9280",
                brand: "samsung",
                manufacturer: "samsung",
                product: "e3q",
                android: "14",
                sdk: "34",
                screen: "1440x3120",
                display: "UP1A.231005.007 release-keys",
                device_info: "samsung/e3q/e3q:14/UP1A.231005.007/S9280ZCU2AXC8:user/release-keys",
                ram: "12288",
                rom: "256",
            },
            Self::OppoFindX7 => DeviceFields {
                model: "PHZ110",
                brand: "OPPO",
                manufacturer: "OPPO",
                product: "PHZ110",
                android: "14",
                sdk: "34",
                screen: "1264x2780",
                display: "UP1A.231005.007 release-keys",
                device_info: "OPPO/PHZ110/PHZ110:14/UP1A.231005.007/T.1dd4c4a:user/release-keys",
                ram: "16384",
                rom: "512",
            },
            Self::VivoX100 => DeviceFields {
                model: "V2309A",
                brand: "vivo",
                manufacturer: "vivo",
                product: "PD2309",
                android: "14",
                sdk: "34",
                screen: "1260x2800",
                display: "UP1A.231005.007 release-keys",
                device_info: "vivo/PD2309/PD2309:14/UP1A.231005.007/14.0.8.0:user/release-keys",
                ram: "16384",
                rom: "512",
            },
            Self::SamsungA52 => DeviceFields {
                model: "SM-A525F",
                brand: "samsung",
                manufacturer: "samsung",
                product: "a52q",
                android: "12",
                sdk: "31",
                screen: "1080x2400",
                display: "SP1A.210812.016 release-keys",
                device_info: "samsung/a52q/a52q:12/SP1A.210812.016/A525FXXU4CVF1:user/release-keys",
                ram: "6144",
                rom: "128",
            },
        }
    }
}

struct DeviceFields {
    model: &'static str,
    brand: &'static str,
    manufacturer: &'static str,
    product: &'static str,
    android: &'static str,
    sdk: &'static str,
    screen: &'static str,
    display: &'static str,
    device_info: &'static str,
    ram: &'static str,
    rom: &'static str,
}

pub fn generate_profile(
    account_id: &AccountId,
    game_biz: GameBiz,
    rng: &mut impl Rng,
) -> DeviceProfile {
    generate_profile_with_template(account_id, game_biz, DeviceTemplate::Xiaomi14, rng)
}

pub fn generate_profile_with_template(
    account_id: &AccountId,
    game_biz: GameBiz,
    template: DeviceTemplate,
    rng: &mut impl Rng,
) -> DeviceProfile {
    let f = template.fields();
    let device_id = format!("{:016x}", rng.gen::<u64>());
    let initial_device_fp = format!("{:013x}", rng.gen::<u64>() & 0x000f_ffff_ffff_ffff);
    let seed_id = Uuid::new_v4().to_string();
    let seed_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_millis()
        .to_string();
    let bbs_device_id = name_uuid_from_bytes(device_id.as_bytes());

    // Synthetic stable OAID/AAID/VAID (not "error_*" so the device looks less broken).
    let oaid: String = (0..32)
        .map(|_| format!("{:x}", rng.gen_range(0..16)))
        .collect();
    let aaid: String = (0..32)
        .map(|_| format!("{:x}", rng.gen_range(0..16)))
        .collect();
    let vaid: String = (0..32)
        .map(|_| format!("{:x}", rng.gen_range(0..16)))
        .collect();

    let mut ext_fields = BTreeMap::new();
    ext_fields.insert("model".to_string(), f.model.to_string());
    ext_fields.insert("brand".to_string(), f.brand.to_string());
    ext_fields.insert("manufacturer".to_string(), f.manufacturer.to_string());
    ext_fields.insert("productName".to_string(), f.product.to_string());
    ext_fields.insert("deviceName".to_string(), f.model.to_string());
    ext_fields.insert("deviceType".to_string(), f.product.to_string());
    ext_fields.insert("deviceInfo".to_string(), f.device_info.to_string());
    ext_fields.insert("osVersion".to_string(), f.android.to_string());
    ext_fields.insert("sdkVersion".to_string(), f.sdk.to_string());
    ext_fields.insert("cpuType".to_string(), "arm64-v8a".to_string());
    ext_fields.insert("screenSize".to_string(), f.screen.to_string());
    ext_fields.insert("display".to_string(), f.display.to_string());
    ext_fields.insert("ramCapacity".to_string(), f.ram.to_string());
    ext_fields.insert("romCapacity".to_string(), f.rom.to_string());
    ext_fields.insert("networkType".to_string(), "WiFi".to_string());
    ext_fields.insert("oaid".to_string(), oaid);
    ext_fields.insert("aaid".to_string(), aaid);
    ext_fields.insert("vaid".to_string(), vaid);
    ext_fields.insert("isRoot".to_string(), "0".to_string());
    ext_fields.insert("emulatorStatus".to_string(), "0".to_string());
    ext_fields.insert("isMockLocation".to_string(), "0".to_string());
    ext_fields.insert("proxyStatus".to_string(), "0".to_string());

    DeviceProfile {
        profile_version: 1,
        account_id: account_id.clone(),
        game_biz,
        client_profile: ClientProfile::miyoushe_2_113_1(),
        device_id,
        seed_id,
        seed_time,
        initial_device_fp,
        bbs_device_id,
        model: f.model.to_string(),
        brand: f.brand.to_string(),
        manufacturer: f.manufacturer.to_string(),
        product_name: f.product.to_string(),
        device_name: f.model.to_string(),
        android_version: f.android.to_string(),
        sdk_version: f.sdk.to_string(),
        abi: "arm64-v8a".to_string(),
        screen_size: f.screen.to_string(),
        display: f.display.to_string(),
        ram_capacity: f.ram.to_string(),
        rom_capacity: f.rom.to_string(),
        network_type: "WiFi".to_string(),
        ext_fields,
    }
}

pub fn name_uuid_from_bytes(name: &[u8]) -> String {
    let mut bytes = Md5::digest(name).to_vec();
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    bytes[0..4].reverse();
    bytes[4..6].reverse();
    bytes[6..8].reverse();
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}
