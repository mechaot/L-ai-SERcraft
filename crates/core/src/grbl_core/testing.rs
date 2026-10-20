//! Test support for the protocol golden tests (not part of the C# code).
//!
//! The C# oracle harness calls the private `ThreadTX` / `ThreadRX` bodies by reflection and opens the
//! connection without starting the threads, so every interleaving is deterministic. These hooks give the
//! Rust tests the same handles. They only call the code the threads call.

use super::types::MacStatus;
use super::GrblCore;
use std::sync::atomic::Ordering;

/// Sizes and flags of the internal queues (what the C# harness reads from `mQueue`, `mPending`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueCounts {
    /// `mQueue.Count`.
    pub queue: usize,
    /// `mPending.Count`.
    pub pending: usize,
    /// `mRetryQueue != null`.
    pub retry: bool,
    /// `mHoldByUserRequest`.
    pub hold_by_user: bool,
    /// `mHoldByCoolingRequest`.
    pub hold_by_cooling: bool,
}

impl GrblCore {
    /// One iteration of the TX thread body (`ThreadTX`).
    #[doc(hidden)]
    pub fn step_tx(&self) {
        self.thread_tx();
    }

    /// One iteration of the RX thread body (`ThreadRX`). Blocks while the link has no line and stays open.
    #[doc(hidden)]
    pub fn step_rx(&self) {
        self.thread_rx();
    }

    /// `OpenCom()` without starting the threads: status, buffer, link open, then the TX first-run part.
    #[doc(hidden)]
    pub fn connect_manual(&self) -> Result<(), String> {
        self.sh.fix_ch340.store(false, Ordering::SeqCst);
        self.sh.ch340_exception.store(0, Ordering::SeqCst);
        self.sh.ch340_goodread.store(0, Ordering::SeqCst);
        {
            let mut st = self.s();
            st.auto_buffer_size = super::state::DEFAULT_BUFFER_SIZE;
            st.set_status(MacStatus::Connecting);
            st.connect_start = st.now_ms();
        }
        let com = self.com();
        if !com.is_open() {
            com.open().map_err(|e| e.to_string())?;
        }
        self.s().start_tx();
        Ok(())
    }

    /// Set the machine status without side effects (no event, no hooks).
    #[doc(hidden)]
    pub fn force_status(&self, s: MacStatus) {
        self.s().machine_status = s;
    }

    /// Start recording status changes (see [`GrblCore::take_status_trace`]).
    #[doc(hidden)]
    pub fn enable_status_trace(&self) {
        self.s().status_trace = Some(Vec::new());
    }

    /// Statuses set since the last call, in order.
    #[doc(hidden)]
    pub fn take_status_trace(&self) -> Vec<MacStatus> {
        self.s().status_trace.as_mut().map(std::mem::take).unwrap_or_default()
    }

    /// Current sleep time of the TX thread object.
    #[doc(hidden)]
    pub fn tx_sleep_time(&self) -> i32 {
        self.sh.tx_h.sleep_time()
    }

    /// Current sleep time of the RX thread object.
    #[doc(hidden)]
    pub fn rx_sleep_time(&self) -> i32 {
        self.sh.rx_h.sleep_time()
    }

    /// Queue sizes and hold flags.
    #[doc(hidden)]
    pub fn queue_counts(&self) -> QueueCounts {
        let st = self.s();
        QueueCounts { queue: st.queue.len(), pending: st.pending.len(), retry: st.retry.is_some(), hold_by_user: st.hold_by_user, hold_by_cooling: st.hold_by_cooling }
    }

    /// `CanSend()`.
    #[doc(hidden)]
    pub fn can_send_now(&self) -> bool {
        self.s().can_send()
    }

    /// The vendor info string seen so far (`mVendorInfoSeen`).
    #[doc(hidden)]
    pub fn vendor_info_seen(&self) -> Option<String> {
        self.s().vendor_info_seen.clone()
    }
}

impl GrblCore {
    /// `mTP.ErrorCount`.
    #[doc(hidden)]
    pub fn program_error_count(&self) -> i32 {
        self.sh.tp().error_count()
    }

    /// `mTP.LastIssue`.
    #[doc(hidden)]
    pub fn last_issue(&self) -> super::types::DetectedIssue {
        self.sh.tp().last_issue()
    }

    /// `mTP.LastKnownWCO`.
    #[doc(hidden)]
    pub fn last_known_wco(&self) -> lg_gcode::GPoint {
        self.sh.tp().last_known_wco()
    }
}

impl GrblCore {
    /// True while a config/info/write operation has swapped in its own queue and sent list.
    #[doc(hidden)]
    pub fn refresh_active(&self) -> bool {
        self.s().alt.is_some()
    }
}
