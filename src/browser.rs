use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::emulation::{
    SetDeviceMetricsOverrideParams, SetUserAgentOverrideParams,
};
use chromiumoxide::cdp::browser_protocol::page::{
    AddScriptToEvaluateOnNewDocumentParams, CaptureScreenshotFormat, NavigateParams,
};
use chromiumoxide::fetcher::{BrowserFetcher, BrowserFetcherOptions};
use futures::StreamExt;
use std::path::PathBuf;

use crate::tools_validation::CaptureParams;
use crate::viewport::ViewportConfig;

const SCROLL_STEP_RATIO: f64 = 0.85;
const SCROLL_PAUSE_MS: u64 = 150;
const SCROLL_MAX_ITERATIONS: u32 = 40;
const SCROLL_SETTLE_MS: u64 = 250;
const NETWORKIDLE_EXTRA_MS: u64 = 800;

const DEFAULT_DESKTOP_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

const STEALTH_SCRIPT: &str = r#"
Object.defineProperty(navigator,'webdriver',{get:()=>undefined});
window.chrome={runtime:{},loadTimes:()=>{},csi:()=>{},app:{}};
Object.defineProperty(navigator,'plugins',{get:()=>[
  {name:'Chrome PDF Plugin',filename:'internal-pdf-viewer',description:'Portable Document Format'},
  {name:'Chrome PDF Viewer',filename:'mhjfbmdgcfjbbpaeojofohoefgiehjai',description:''},
  {name:'Native Client',filename:'internal-nacl-plugin',description:''}
]});
Object.defineProperty(navigator,'languages',{get:()=>['en-US','en']});
"#;

pub struct ScreenshotResult {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: String,
}

pub async fn capture(params: &CaptureParams) -> Result<ScreenshotResult> {
    let viewport = ViewportConfig::from_preset(&params.viewport, params.width, params.height)?;
    let chrome_path = resolve_chrome().await?;

    let (mut browser, mut handler) = Browser::launch(
        BrowserConfig::builder()
            .chrome_executable(chrome_path)
            .window_size(viewport.width, viewport.height)
            .no_sandbox()
            .arg("--disable-dev-shm-usage")
            .arg("--disable-background-timer-throttling")
            .arg("--disable-renderer-backgrounding")
            .arg("--disable-backgrounding-occluded-windows")
            .arg("--disable-blink-features=AutomationControlled")
            .build()
            .map_err(|e| anyhow!("browser config: {e}"))?,
    )
    .await
    .context("failed to launch Chrome")?;

    let handler_task = tokio::spawn(async move { while handler.next().await.is_some() {} });

    let result = do_capture(&mut browser, params, &viewport).await;

    browser.close().await.ok();
    handler_task.abort();

    result
}

async fn do_capture(
    browser: &mut Browser,
    params: &CaptureParams,
    viewport: &ViewportConfig,
) -> Result<ScreenshotResult> {
    let page = browser
        .new_page("about:blank")
        .await
        .context("failed to open page")?;

    inject_stealth(&page).await?;

    apply_device_metrics(
        &page,
        viewport.width,
        viewport.height,
        viewport.device_scale_factor,
        viewport.mobile,
    )
    .await?;

    let ua = viewport.user_agent.as_deref().unwrap_or(DEFAULT_DESKTOP_UA);
    apply_user_agent(&page, ua).await?;

    page.execute(
        NavigateParams::builder()
            .url(&params.url)
            .build()
            .map_err(|e| anyhow!("navigate params: {e}"))?,
    )
    .await
    .context("navigate to target")?;

    wait_for_page(&page, &params.wait_strategy).await?;

    if let Some(extra_ms) = extract_delay_ms(&params.wait_strategy) {
        tokio::time::sleep(tokio::time::Duration::from_millis(extra_ms)).await;
    }

    if params.full_page {
        scroll_for_lazy_load(&page, viewport.height).await.ok();
    }

    let (capture_width, capture_height) = if params.full_page {
        let full_h = get_scroll_height(&page).await?;
        if full_h > viewport.height {
            apply_device_metrics(
                &page,
                viewport.width,
                full_h,
                viewport.device_scale_factor,
                viewport.mobile,
            )
            .await
            .ok();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        (viewport.width, full_h)
    } else {
        (viewport.width, viewport.height)
    };

    let fmt = if params.format == "jpeg" {
        CaptureScreenshotFormat::Jpeg
    } else {
        CaptureScreenshotFormat::Png
    };

    let screenshot_data = page
        .screenshot(
            chromiumoxide::page::ScreenshotParams::builder()
                .format(fmt)
                .full_page(params.full_page)
                .build(),
        )
        .await
        .context("screenshot capture")?;

    Ok(ScreenshotResult {
        data: screenshot_data,
        width: capture_width,
        height: capture_height,
        format: params.format.clone(),
    })
}

async fn inject_stealth(page: &chromiumoxide::Page) -> Result<()> {
    page.execute(
        AddScriptToEvaluateOnNewDocumentParams::builder()
            .source(STEALTH_SCRIPT)
            .build()
            .map_err(|e| anyhow!("stealth params: {e}"))?,
    )
    .await
    .context("inject stealth script")?;
    Ok(())
}

async fn scroll_for_lazy_load(page: &chromiumoxide::Page, viewport_height: u32) -> Result<()> {
    let step = (viewport_height as f64 * SCROLL_STEP_RATIO) as u64;
    let mut position: u64 = 0;
    let mut iterations: u32 = 0;

    loop {
        let total = page
            .evaluate("document.documentElement.scrollHeight")
            .await
            .map(|v| v.into_value::<f64>().unwrap_or(0.0) as u64)
            .unwrap_or(0);

        if position >= total || iterations >= SCROLL_MAX_ITERATIONS {
            break;
        }

        position = (position + step).min(total);
        page.evaluate(format!(
            "window.scrollTo({{top:{},behavior:'instant'}})",
            position
        ))
        .await
        .ok();

        tokio::time::sleep(tokio::time::Duration::from_millis(SCROLL_PAUSE_MS)).await;
        iterations += 1;
    }

    page.evaluate("window.scrollTo({top:0,behavior:'instant'})")
        .await
        .ok();
    tokio::time::sleep(tokio::time::Duration::from_millis(SCROLL_SETTLE_MS)).await;
    Ok(())
}

async fn wait_for_page(page: &chromiumoxide::Page, strategy: &str) -> Result<()> {
    page.wait_for_navigation()
        .await
        .context("wait for navigation")?;

    if strategy == "networkidle" {
        tokio::time::sleep(tokio::time::Duration::from_millis(NETWORKIDLE_EXTRA_MS)).await;
    }
    Ok(())
}

async fn get_scroll_height(page: &chromiumoxide::Page) -> Result<u32> {
    let height: f64 = page
        .evaluate("Math.max(document.body.scrollHeight, document.documentElement.scrollHeight)")
        .await
        .context("get scroll height")?
        .into_value()
        .unwrap_or(900.0_f64);
    Ok(height as u32)
}

async fn apply_device_metrics(
    page: &chromiumoxide::Page,
    width: u32,
    height: u32,
    scale: f64,
    mobile: bool,
) -> Result<()> {
    let metrics = SetDeviceMetricsOverrideParams::builder()
        .width(width)
        .height(height)
        .device_scale_factor(scale)
        .mobile(mobile)
        .build()
        .map_err(|e| anyhow!("metrics: {e}"))?;
    page.execute(metrics).await.context("set device metrics")?;
    Ok(())
}

async fn apply_user_agent(page: &chromiumoxide::Page, ua: &str) -> Result<()> {
    let ua_params = SetUserAgentOverrideParams::builder()
        .user_agent(ua)
        .build()
        .map_err(|e| anyhow!("user agent: {e}"))?;
    page.execute(ua_params).await.context("set user agent")?;
    Ok(())
}

fn extract_delay_ms(strategy: &str) -> Option<u64> {
    strategy
        .strip_prefix("delay:")
        .and_then(|s| s.parse::<u64>().ok())
}

pub async fn resolve_chrome() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("CHROME_PATH") {
        let p = PathBuf::from(&path);
        if p.exists() {
            return Ok(p);
        }
        return Err(anyhow!("CHROME_PATH='{}' does not exist", path));
    }

    if let Some(p) = find_system_chrome() {
        return Ok(p);
    }

    download_chrome().await
}

fn find_system_chrome() -> Option<PathBuf> {
    for name in chrome_candidates() {
        let path = PathBuf::from(&name);
        if path.is_absolute() && path.exists() {
            return Some(path);
        }
        if let Ok(p) = which::which(&name) {
            return Some(p);
        }
    }
    None
}

async fn download_chrome() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("screenshot-mcp")
        .join("chrome");

    tokio::fs::create_dir_all(&cache_dir)
        .await
        .with_context(|| format!("create cache dir: {}", cache_dir.display()))?;

    eprintln!(
        "screenshot-mcp: Chrome not found, downloading to {} (first run only)...",
        cache_dir.display()
    );

    let fetcher = BrowserFetcher::new(
        BrowserFetcherOptions::builder()
            .with_path(&cache_dir)
            .build()
            .map_err(|e| anyhow!("fetcher options: {e}"))?,
    );

    let info = fetcher
        .fetch()
        .await
        .context("Chrome download failed — ensure internet access on first run")?;

    eprintln!(
        "screenshot-mcp: Chrome ready at {}",
        info.executable_path.display()
    );

    Ok(info.executable_path)
}

fn chrome_candidates() -> Vec<String> {
    let mut candidates = vec![
        "google-chrome".to_string(),
        "google-chrome-stable".to_string(),
        "chromium".to_string(),
        "chromium-browser".to_string(),
    ];

    #[cfg(target_os = "macos")]
    candidates.push("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".to_string());

    #[cfg(target_os = "windows")]
    {
        candidates.push(r"C:\Program Files\Google\Chrome\Application\chrome.exe".to_string());
        candidates.push(r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe".to_string());
    }

    candidates
}

pub fn encode_base64(data: &[u8]) -> String {
    B64.encode(data)
}

pub fn save_to_file(data: &[u8], path: &str) -> Result<()> {
    std::fs::write(path, data).with_context(|| format!("write screenshot to {path}"))?;
    Ok(())
}
