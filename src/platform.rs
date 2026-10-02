//! Small platform shims: timers, monotonic clock, blocking work.

#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep_ms(ms: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
}

#[cfg(target_arch = "wasm32")]
pub async fn sleep_ms(ms: u64) {
    gloo_timers::future::TimeoutFuture::new(ms.min(u32::MAX as u64) as u32).await;
}

/// Runs CPU-bound work (script evaluation) off the UI thread where possible.
/// The browser has no threads available to us here, so on web it runs inline.
#[cfg(not(target_arch = "wasm32"))]
pub async fn run_blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    match tokio::runtime::Handle::try_current() {
        Ok(h) => h.spawn_blocking(f).await.expect("blocking task panicked"),
        Err(_) => f(),
    }
}

#[cfg(target_arch = "wasm32")]
pub async fn run_blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    f()
}

#[derive(Clone, Copy)]
pub struct Instant(f64);

impl Instant {
    pub fn now() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            Instant(js_sys::Date::now())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::sync::OnceLock;
            static START: OnceLock<std::time::Instant> = OnceLock::new();
            let start = START.get_or_init(std::time::Instant::now);
            Instant(start.elapsed().as_secs_f64() * 1000.0)
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        (Self::now().0 - self.0).max(0.0) as u64
    }
}
