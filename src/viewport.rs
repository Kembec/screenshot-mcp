#[derive(Debug, Clone)]
pub struct ViewportConfig {
    pub width: u32,
    pub height: u32,
    pub device_scale_factor: f64,
    pub mobile: bool,
    pub user_agent: Option<String>,
}

const MOBILE_UA: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1";

impl ViewportConfig {
    pub fn from_preset(
        preset: &str,
        width: Option<u32>,
        height: Option<u32>,
    ) -> anyhow::Result<Self> {
        match preset {
            "desktop" => Ok(Self {
                width: 1440,
                height: 900,
                device_scale_factor: 1.0,
                mobile: false,
                user_agent: None,
            }),
            "laptop" => Ok(Self {
                width: 1280,
                height: 800,
                device_scale_factor: 1.0,
                mobile: false,
                user_agent: None,
            }),
            "tablet" => Ok(Self {
                width: 768,
                height: 1024,
                device_scale_factor: 2.0,
                mobile: false,
                user_agent: None,
            }),
            "mobile" => Ok(Self {
                width: 390,
                height: 844,
                device_scale_factor: 3.0,
                mobile: true,
                user_agent: Some(MOBILE_UA.to_string()),
            }),
            "mobile_landscape" => Ok(Self {
                width: 844,
                height: 390,
                device_scale_factor: 3.0,
                mobile: true,
                user_agent: Some(MOBILE_UA.to_string()),
            }),
            "custom" => {
                let w = width
                    .ok_or_else(|| anyhow::anyhow!("width required for custom viewport"))?;
                let h = height
                    .ok_or_else(|| anyhow::anyhow!("height required for custom viewport"))?;
                Ok(Self {
                    width: w,
                    height: h,
                    device_scale_factor: 1.0,
                    mobile: false,
                    user_agent: None,
                })
            }
            other => Err(anyhow::anyhow!("unknown viewport preset: {other}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_preset() {
        let v = ViewportConfig::from_preset("desktop", None, None).unwrap();
        assert_eq!(v.width, 1440);
        assert!(!v.mobile);
    }

    #[test]
    fn mobile_preset_has_ua() {
        let v = ViewportConfig::from_preset("mobile", None, None).unwrap();
        assert_eq!(v.width, 390);
        assert!(v.mobile);
        assert!(v.user_agent.is_some());
    }

    #[test]
    fn custom_requires_width_height() {
        assert!(ViewportConfig::from_preset("custom", None, None).is_err());
        assert!(ViewportConfig::from_preset("custom", Some(1024), Some(768)).is_ok());
    }
}
