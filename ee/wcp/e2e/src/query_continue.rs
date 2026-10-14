//! Stand-in for the `IQueryContinueWithStatus` LogonUI passes to `Connect`,
//! covering both "let it finish" and "user backed out of the tile".

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use windows::{
    Win32::{
        Foundation::E_ABORT,
        UI::Shell::{IQueryContinue_Impl, IQueryContinueWithStatus, IQueryContinueWithStatus_Impl},
    },
    core::{PCWSTR, Result, implement},
};

#[derive(Default)]
struct State {
    polls: AtomicU32,
    cancelled: AtomicBool,
    status: Mutex<Vec<String>>,
}

/// Reads the state of a live `FakeQueryContinue`, which the interface itself
/// gives no way back to.
#[derive(Clone)]
pub struct QueryContinueProbe(Arc<State>);

impl QueryContinueProbe {
    /// Messages `Connect` pushed through `SetStatusMessage`.
    pub fn status_messages(&self) -> Vec<String> {
        self.0
            .status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn polls(&self) -> u32 {
        self.0.polls.load(Ordering::SeqCst)
    }

    pub fn cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::SeqCst)
    }
}

#[implement(IQueryContinueWithStatus)]
struct FakeQueryContinue {
    cancel_after: Option<u32>,
    state: Arc<State>,
}

/// `cancel_after` is how many `QueryContinue` calls to allow before reporting
/// cancellation; `None` runs to completion.
pub fn query_continue(cancel_after: Option<u32>) -> (IQueryContinueWithStatus, QueryContinueProbe) {
    let state = Arc::new(State::default());
    let com = FakeQueryContinue {
        cancel_after,
        state: state.clone(),
    }
    .into();
    (com, QueryContinueProbe(state))
}

impl IQueryContinue_Impl for FakeQueryContinue_Impl {
    fn QueryContinue(&self) -> Result<()> {
        let seen = self.state.polls.fetch_add(1, Ordering::SeqCst) + 1;
        match self.cancel_after {
            Some(limit) if seen > limit => {
                self.state.cancelled.store(true, Ordering::SeqCst);
                Err(E_ABORT.into())
            }
            _ => Ok(()),
        }
    }
}

impl IQueryContinueWithStatus_Impl for FakeQueryContinue_Impl {
    fn SetStatusMessage(&self, psz: &PCWSTR) -> Result<()> {
        let message = if psz.is_null() {
            String::new()
        } else {
            unsafe { psz.to_string() }.unwrap_or_default()
        };
        self.state
            .status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(message);
        Ok(())
    }
}
