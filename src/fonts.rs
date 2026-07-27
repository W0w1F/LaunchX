use egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

/// Candidate system fonts with CJK coverage, in preference order.
/// (family name, paths to try)
#[cfg(windows)]
const CANDIDATES: &[&str] = &[
    "C:/Windows/Fonts/msyh.ttc",    // Microsoft YaHei (Simplified Chinese)
    "C:/Windows/Fonts/msyh.ttf",
    "C:/Windows/Fonts/msjh.ttc",    // Microsoft JhengHei (Traditional Chinese)
    "C:/Windows/Fonts/simsun.ttc",  // SimSun
    "C:/Windows/Fonts/meiryo.ttc",  // Meiryo (Japanese)
    "C:/Windows/Fonts/malgun.ttf",  // Malgun Gothic (Korean)
];

#[cfg(target_os = "macos")]
const CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/PingFang.ttc",
    "/System/Library/Fonts/STHeiti Light.ttc",
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
    "/Library/Fonts/Arial Unicode.ttf",
];

#[cfg(all(unix, not(target_os = "macos")))]
const CANDIDATES: &[&str] = &[
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/opentype/source-han-sans/SourceHanSans-Regular.otf",
    "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
    "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
    "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
];

/// Install a system CJK font as a fallback so Chinese/Japanese/Korean
/// text (e.g. file paths) renders instead of showing tofu (□).
/// Returns the path of the font that was loaded, if any.
pub fn install_cjk_fallback(ctx: &egui::Context) -> Option<&'static str> {
    for path in CANDIDATES {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = FontDefinitions::default();
            fonts
                .font_data
                .insert("cjk".to_owned(), Arc::new(FontData::from_owned(bytes)));
            // Append as last-resort fallback for both families.
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .push("cjk".to_owned());
            fonts
                .families
                .entry(FontFamily::Monospace)
                .or_default()
                .push("cjk".to_owned());
            ctx.set_fonts(fonts);
            return Some(path);
        }
    }
    None
}
