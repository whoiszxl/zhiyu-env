use qrcode::{render::svg, EcLevel, QrCode};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrCodeResult {
    svg: String,
    modules: usize,
    content_bytes: usize,
    version: usize,
}

#[tauri::command]
pub async fn qr_code_generate(
    content: String,
    error_correction: String,
    size: u32,
    foreground: String,
    background: String,
    quiet_zone: bool,
) -> Result<QrCodeResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        generate(
            &content,
            &error_correction,
            size,
            &foreground,
            &background,
            quiet_zone,
        )
    })
    .await
    .map_err(|error| format!("二维码生成任务异常：{error}"))?
}

fn generate(
    content: &str,
    error_correction: &str,
    size: u32,
    foreground: &str,
    background: &str,
    quiet_zone: bool,
) -> Result<QrCodeResult, String> {
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
    validate_color(foreground, false)?;
    validate_color(background, true)?;
    if background != "transparent" && foreground.eq_ignore_ascii_case(background) {
        return Err("二维码前景色和背景色不能相同".into());
    }
    let code = QrCode::with_error_correction_level(content.as_bytes(), level)
        .map_err(|error| format!("无法生成二维码，内容可能过长：{error}"))?;
    let modules = code.width();
    let background = if background == "transparent" {
        "none"
    } else {
        background
    };
    let svg = code
        .render::<svg::Color>()
        .min_dimensions(size, size)
        .dark_color(svg::Color(foreground))
        .light_color(svg::Color(background))
        .quiet_zone(quiet_zone)
        .build();
    Ok(QrCodeResult {
        svg,
        modules,
        content_bytes: content.len(),
        version: (modules.saturating_sub(17)) / 4,
    })
}

fn validate_color(value: &str, transparent_allowed: bool) -> Result<(), String> {
    if transparent_allowed && value == "transparent" {
        return Ok(());
    }
    if value.len() == 7
        && value.starts_with('#')
        && value[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Ok(())
    } else {
        Err("二维码颜色必须使用 #RRGGBB 格式".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_svg_for_utf8_content() {
        let result = generate(
            "https://zhiyu.dev/你好",
            "M",
            320,
            "#1f231d",
            "#ffffff",
            true,
        )
        .expect("应生成二维码");
        assert!(result.svg.starts_with("<?xml"));
        assert!(result.svg.contains("<svg"));
        assert!(result.modules >= 21);
        assert!(result.version >= 1);
    }

    #[test]
    fn rejects_empty_and_invalid_options() {
        assert!(generate("", "M", 320, "#000000", "#ffffff", true).is_err());
        assert!(generate("hello", "X", 320, "#000000", "#ffffff", true).is_err());
        assert!(generate("hello", "M", 100, "#000000", "#ffffff", true).is_err());
        assert!(generate("hello", "M", 320, "red", "#ffffff", true).is_err());
        assert!(generate("hello", "M", 320, "#ffffff", "#ffffff", true).is_err());
    }

    #[test]
    fn supports_transparent_background() {
        let result = generate("hello", "Q", 320, "#112233", "transparent", true).unwrap();
        assert!(result.svg.contains("fill=\"none\""));
    }
}
