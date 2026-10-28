use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use laputa_render::{
    FfmpegPreviewDecoder, PreviewDecodeRequest, PreviewDecodeSession, PreviewFrame,
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PreviewPlaybackEvent {
    Frame(PreviewFrame),
    Finished,
    Failed(String),
}

#[derive(Debug)]
pub struct PreviewPlaybackHandle {
    cancelled: Arc<AtomicBool>,
}

impl PreviewPlaybackHandle {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

impl Drop for PreviewPlaybackHandle {
    fn drop(&mut self) {
        self.cancel();
    }
}

pub fn spawn_preview_playback(
    decoder: FfmpegPreviewDecoder,
    request: PreviewDecodeRequest,
) -> Result<(
    PreviewPlaybackHandle,
    UnboundedReceiver<PreviewPlaybackEvent>,
)> {
    request.validate()?;
    let cancelled = Arc::new(AtomicBool::new(false));
    let worker_cancelled = cancelled.clone();
    let (sender, receiver) = unbounded_channel();

    thread::Builder::new()
        .name("laputa-preview-playback".to_owned())
        .spawn(move || {
            let start_ms = request.start_ms;
            match decoder.open(request) {
                Ok(mut session) => {
                    if let Err(error) =
                        drive_session(&mut session, start_ms, &worker_cancelled, &sender)
                        && !worker_cancelled.load(Ordering::Acquire)
                    {
                        let _ = sender.send(PreviewPlaybackEvent::Failed(error.to_string()));
                    }
                }
                Err(error) => {
                    if !worker_cancelled.load(Ordering::Acquire) {
                        let _ = sender.send(PreviewPlaybackEvent::Failed(error.to_string()));
                    }
                }
            }
        })
        .context("failed to start preview playback worker")?;

    Ok((PreviewPlaybackHandle { cancelled }, receiver))
}

trait PlaybackSession {
    fn read_frame(&mut self) -> Result<Option<PreviewFrame>>;
    fn stop(&mut self) -> Result<()>;
}

impl PlaybackSession for PreviewDecodeSession {
    fn read_frame(&mut self) -> Result<Option<PreviewFrame>> {
        PreviewDecodeSession::read_frame(self)
    }

    fn stop(&mut self) -> Result<()> {
        PreviewDecodeSession::stop(self)
    }
}

fn drive_session<S: PlaybackSession>(
    session: &mut S,
    start_ms: u64,
    cancelled: &AtomicBool,
    sender: &UnboundedSender<PreviewPlaybackEvent>,
) -> Result<()> {
    let started = Instant::now();

    loop {
        if cancelled.load(Ordering::Acquire) {
            session.stop()?;
            return Ok(());
        }

        let Some(frame) = session.read_frame()? else {
            if !cancelled.load(Ordering::Acquire) {
                let _ = sender.send(PreviewPlaybackEvent::Finished);
            }
            return Ok(());
        };

        pace_until(
            started,
            frame.timeline_ms.saturating_sub(start_ms),
            cancelled,
        );
        if cancelled.load(Ordering::Acquire) {
            session.stop()?;
            return Ok(());
        }
        if sender.send(PreviewPlaybackEvent::Frame(frame)).is_err() {
            session.stop()?;
            return Ok(());
        }
    }
}

fn pace_until(started: Instant, offset_ms: u64, cancelled: &AtomicBool) {
    let target = started + Duration::from_millis(offset_ms);
    loop {
        if cancelled.load(Ordering::Acquire) {
            return;
        }
        let now = Instant::now();
        if now >= target {
            return;
        }
        let remaining = target.saturating_duration_since(now);
        thread::sleep(remaining.min(Duration::from_millis(10)));
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use anyhow::Result;

    use super::{PlaybackSession, PreviewPlaybackEvent, drive_session};
    use laputa_render::PreviewFrame;
    use tokio::sync::mpsc::unbounded_channel;

    struct FakeSession {
        frames: VecDeque<PreviewFrame>,
        stopped: bool,
    }

    impl FakeSession {
        fn new(frames: Vec<PreviewFrame>) -> Self {
            Self {
                frames: frames.into(),
                stopped: false,
            }
        }
    }

    impl PlaybackSession for FakeSession {
        fn read_frame(&mut self) -> Result<Option<PreviewFrame>> {
            Ok(self.frames.pop_front())
        }

        fn stop(&mut self) -> Result<()> {
            self.stopped = true;
            Ok(())
        }
    }

    fn frame(timeline_ms: u64, value: u8) -> PreviewFrame {
        PreviewFrame {
            timeline_ms,
            width: 1,
            height: 1,
            rgba: vec![value, value, value, 255],
        }
    }

    #[test]
    fn playback_emits_frames_in_order_then_finished() -> Result<()> {
        let mut session = FakeSession::new(vec![frame(1_000, 1), frame(1_001, 2)]);
        let cancelled = std::sync::atomic::AtomicBool::new(false);
        let (sender, mut receiver) = unbounded_channel();

        drive_session(&mut session, 1_000, &cancelled, &sender)?;

        assert!(matches!(
            receiver.try_recv(),
            Ok(PreviewPlaybackEvent::Frame(PreviewFrame {
                timeline_ms: 1_000,
                ..
            }))
        ));
        assert!(matches!(
            receiver.try_recv(),
            Ok(PreviewPlaybackEvent::Frame(PreviewFrame {
                timeline_ms: 1_001,
                ..
            }))
        ));
        assert_eq!(receiver.try_recv(), Ok(PreviewPlaybackEvent::Finished));
        assert!(!session.stopped);
        Ok(())
    }

    #[test]
    fn cancelled_playback_stops_source_without_finished_event() -> Result<()> {
        let mut session = FakeSession::new(vec![frame(1_000, 1)]);
        let cancelled = std::sync::atomic::AtomicBool::new(true);
        let (sender, mut receiver) = unbounded_channel();

        drive_session(&mut session, 1_000, &cancelled, &sender)?;

        assert!(session.stopped);
        assert!(receiver.try_recv().is_err());
        Ok(())
    }
}
