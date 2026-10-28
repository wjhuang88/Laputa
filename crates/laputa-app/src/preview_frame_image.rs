use std::sync::Arc;

use anyhow::{Context, Result};
use gpui::RenderImage;
use image::{Frame, RgbaImage};
use laputa_render::PreviewFrame;

#[derive(Clone, Debug)]
pub struct RenderedPreviewFrame {
    pub timeline_ms: u64,
    pub image: Arc<RenderImage>,
}

pub fn into_rendered_preview_frame(frame: PreviewFrame) -> Result<RenderedPreviewFrame> {
    let timeline_ms = frame.timeline_ms;
    let buffer = RgbaImage::from_raw(frame.width, frame.height, frame.rgba)
        .context("preview RGBA buffer length does not match its dimensions")?;
    let image = Arc::new(RenderImage::new(vec![Frame::new(buffer)]));
    Ok(RenderedPreviewFrame { timeline_ms, image })
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use laputa_render::PreviewFrame;

    use super::into_rendered_preview_frame;

    #[test]
    fn rgba_frame_becomes_single_gpui_render_frame_without_reencoding() -> Result<()> {
        let rendered = into_rendered_preview_frame(PreviewFrame {
            timeline_ms: 1_250,
            width: 2,
            height: 1,
            rgba: vec![255, 0, 0, 255, 0, 255, 0, 255],
        })?;

        assert_eq!(rendered.timeline_ms, 1_250);
        assert_eq!(rendered.image.frame_count(), 1);
        assert_eq!(rendered.image.as_bytes(0).map(<[u8]>::len), Some(8));
        Ok(())
    }
}
