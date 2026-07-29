use qrcode::{render::svg, EcLevel, QrCode};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrCodeResult {
    svg: String,
    modules: usize,
    content_bytes: usize,
}

#[tauri::command]
pub async fn qr_code_generate(
    content: String,
    error_correction: String,
    size: u32,
) -> Result<QrCodeResult, String> {
    tauri::async_runtime::spawn_blocking(move || generate(&content, &error_correction, size))
        .await
        .map_err(|error| format!("二维码生成任务异常：{error}"))?
}

fn generate(content: &str, error_correction: &str, size: u32) -> Result<QrCodeResult, String> {
    if content.is_empty() {
        return Err("请输入需要生成二维码的内容".into());
    }
    if content.len() > 4096 {
        return Err("二维码内容不能超过 4096 字节".into());
    }
    if !(160..=1024).contains(&size) {
        return Err("二维码尺寸必须在 160 到 1024 像素之间".into());
    }
    let level = match error_correction.to_ascii_uppercase().as_str() {
        "L" => EcLevel::L,
        "M" => EcLevel::M,
        "Q" => EcLevel::Q,
        "H" => EcLevel::H,
        _ => return Err("纠错等级只支持 L、M、Q、H".into()),
    };
    let code = QrCode::with_error_correction_level(content.as_bytes(), level)
        .map_err(|error| format!("无法生成二维码，内容可能过长：{error}"))?;
    let modules = code.width();
    let svg = code
        .render::<svg::Color>()
        .min_dimensions(size, size)
        .dark_color(svg::Color("#1f231d"))
        .light_color(svg::Color("#ffffff"))
        .quiet_zone(true)
        .build();
    Ok(QrCodeResult {
        svg,
        modules,
        content_bytes: content.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_svg_for_utf8_content() {
        let result = generate("https://zhiyu.dev/你好", "M", 320).expect("应生成二维码");
        assert!(result.svg.starts_with("<?xml"));
        assert!(result.svg.contains("<svg"));
        assert!(result.modules >= 21);
    }

    #[test]
    fn rejects_empty_and_invalid_options() {
        assert!(generate("", "M", 320).is_err());
        assert!(generate("hello", "X", 320).is_err());
        assert!(generate("hello", "M", 100).is_err());
    }
}
