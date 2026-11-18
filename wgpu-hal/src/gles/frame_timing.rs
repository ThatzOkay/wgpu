#[cfg(gles_with_std)]
mod native {
    use std::{
        sync::{
            atomic::{AtomicU64, Ordering},
            OnceLock,
        },
        time::{Instant, SystemTime, UNIX_EPOCH},
    };

    pub(crate) struct Timer(Option<Sample>);
    struct Sample {
        id: u64,
        scope: &'static str,
        start_us: u128,
        start: Instant,
        last: Instant,
        stages: [(&'static str, f64); 12],
        count: usize,
    }
    impl Timer {
        pub(crate) fn new(scope: &'static str) -> Self {
            static ENABLED: OnceLock<bool> = OnceLock::new();
            static NEXT: AtomicU64 = AtomicU64::new(1);
            if !*ENABLED.get_or_init(|| {
                matches!(
                    std::env::var("WGPU_FRAME_TIMING").as_deref(),
                    Ok("1" | "true")
                )
            }) {
                return Self(None);
            }
            let now = Instant::now();
            Self(Some(Sample {
                id: NEXT.fetch_add(1, Ordering::Relaxed),
                scope,
                start_us: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_micros(),
                start: now,
                last: now,
                stages: [("", 0.0); 12],
                count: 0,
            }))
        }
        pub(crate) fn mark(&mut self, stage: &'static str) {
            if let Some(s) = &mut self.0 {
                let now = Instant::now();
                if s.count < s.stages.len() {
                    s.stages[s.count] = (stage, now.duration_since(s.last).as_secs_f64() * 1000.0);
                    s.count += 1;
                }
                s.last = now;
            }
        }
    }
    impl Drop for Timer {
        fn drop(&mut self) {
            self.mark("tail");
            if let Some(s) = &self.0 {
                log::info!(target: "wgpu_frame_timing", "scope={} id={} start_us={} total_ms={:.3} stages_ms={:?}",
                s.scope, s.id, s.start_us, s.start.elapsed().as_secs_f64() * 1000.0, &s.stages[..s.count]);
            }
        }
    }
}
#[cfg(gles_with_std)]
pub(super) use native::Timer;
#[cfg(not(gles_with_std))]
pub(super) struct Timer;
#[cfg(not(gles_with_std))]
impl Timer {
    pub(super) fn new(_: &'static str) -> Self {
        Self
    }
    pub(super) fn mark(&mut self, _: &'static str) {}
}

#[cfg(gles_with_std)]
pub(super) unsafe fn dump_frame(gl: &glow::Context, extent: wgt::Extent3d) {
    use glow::HasContext;
    use std::{
        io::Write,
        sync::{
            atomic::{AtomicU32, Ordering},
            OnceLock,
        },
    };
    static PATH: OnceLock<Option<std::string::String>> = OnceLock::new();
    static FRAME: AtomicU32 = AtomicU32::new(0);
    let Some(path) = PATH.get_or_init(|| std::env::var("WGPU_GLES_FRAME_DUMP").ok()) else {
        return;
    };
    if FRAME.fetch_add(1, Ordering::Relaxed) != 9 {
        return;
    }
    let mut pixels = std::vec![0u8; extent.width as usize * extent.height as usize * 4];
    unsafe {
        gl.read_pixels(
            0,
            0,
            extent.width as i32,
            extent.height as i32,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelPackData::Slice(Some(&mut pixels)),
        )
    };
    let result = (|| -> std::io::Result<()> {
        let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
        writeln!(file, "P6\n{} {}\n255", extent.width, extent.height)?;
        let mut row = std::vec::Vec::with_capacity(extent.width as usize * 3);
        for rgba in pixels.chunks_exact(extent.width as usize * 4).rev() {
            row.clear();
            for pixel in rgba.chunks_exact(4) {
                row.extend_from_slice(&pixel[..3]);
            }
            file.write_all(&row)?;
        }
        file.flush()
    })();
    match result {
        Ok(()) => log::info!("Captured GLES frame to {path}"),
        Err(error) => log::warn!("GLES frame capture failed: {error}"),
    }
}
