//! A stand-in for the `IQueryContinueWithStatus` LogonUI passes to
//! `Connect`, so tests can drive both the "let it finish" and the "user
//! backed out of the tile" paths.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use windows::{
    Win32::{
        Foundation::E_ABORT,
        UI::Shell::{IQueryContinue_Impl, IQueryContinueWithStatus, IQueryContinueWithStatus_Impl},
    },
    core::{PCWSTR, Result, implement},
};

/// Handle onto the state of a live `SharedQueryContinue`, since the interface
/// itself hands out no way back to the implementation.
#[derive(Clone)]
pub struct QueryContinueProbe {
    inner: Arc<Shared>,
}

#[derive(Default)]
struct Shared {
    polls: AtomicU32,
    cancelled: AtomicBool,
    status: Mutex<Vec<String>>,
}

impl QueryContinueProbe {
    /// Status messages `Connect` pushed through `SetStatusMessage`.
    pub fn status_messages(&self) -> Vec<String> {
        self.inner
            .status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn polls(&self) -> u32 {
        self.inner.polls.load(Ordering::SeqCst)
    }

    pub fn cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }
}

#[implement(IQueryContinueWithStatus)]
pub struct SharedQueryContinue {
    cancel_after: Option<u32>,
    inner: Arc<Shared>,
}

/// Builds an `IQueryContinueWithStatus` plus a probe onto its state.
/// `cancel_after` is the number of `QueryContinue` calls to allow before
/// reporting cancellation; `None` lets the flow run to completion.
pub fn query_continue(cancel_after: Option<u32>) -> (IQueryContinueWithStatus, QueryContinueProbe) {
    let inner = Arc::new(Shared::default());
    let probe = QueryContinueProbe {
        inner: inner.clone(),
    };
    let com: IQueryContinueWithStatus = SharedQueryContinue {
        cancel_after,
        inner,
    }
    .into();
    (com, probe)
}

impl IQueryContinue_Impl for SharedQueryContinue_Impl {
    fn QueryContinue(&self) -> Result<()> {
        let seen = self.inner.polls.fetch_add(1, Ordering::SeqCst) + 1;
        match self.cancel_after {
            Some(limit) if seen > limit => {
                self.inner.cancelled.store(true, Ordering::SeqCst);
                Err(E_ABORT.into())
            }
            _ => Ok(()),
        }
    }
}

impl IQueryContinueWithStatus_Impl for SharedQueryContinue_Impl {
    fn SetStatusMessage(&self, psz: &PCWSTR) -> Result<()> {
        let message = if psz.is_null() {
            String::new()
        } else {
            unsafe { psz.to_string() }.unwrap_or_default()
        };
        self.inner
            .status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(message);
        Ok(())
    }
}
