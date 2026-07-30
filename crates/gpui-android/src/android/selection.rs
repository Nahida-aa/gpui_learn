//! System Android `ActionMode` selection-toolbar bridge.
//!
//! This module is the Rust half of the *system* selection toolbar (approach
//! B chosen by the user): instead of drawing our own floating action bar, we
//! let Android show its native `ActionMode` above the selection, and route its
//! menu clicks back to the focused editor.
//!
//! Responsibilities split cleanly:
//!
//! 1. **Detection** — `window.rs` asks the focused `EntityInputHandler` for its
//!    current selection each frame. When it transitions empty↔non-empty we
//!    call [`start_action_mode`] / [`finish_action_mode`] (Rust→Java).
//!
//! 2. **Routing** — Java's `ActionMode.Callback` calls `nativeSelectionAction`
//!    (Java→Rust, see `jni.rs`) with a verb code. We enqueue it onto
//!    [`SELECTION_COMMANDS`] and drain it on the GPUI thread
//!    ([`drain_selection_commands`]), forwarding to the app-supplied
//!    [`SelectionHandler`].
//!
//! The actual copy/cut/paste/select-all *logic* lives in the app (e.g. 06's
//! `Editor`), because only it knows the focused entity and has clipboard
//! access. gpui-android stays generic and only brokers the toolbar + clicks.
//!
//! Desktop is untouched: this code is compiled only for `target_os = "android"`
//! and nothing calls into it from non-Android builds.

#![allow(unsafe_code)]

use std::sync::{Arc, OnceLock};

// Re-use jni.rs helpers for the Rust→Java calls.
use super::jni::{activity, with_env, JniExt};
use gpui::App;

/// Verb codes shared with Java's `ActionMode.Callback`.
///
/// Keep these in sync with the `nativeSelectionAction` switch in
/// `GpuiActivity.java`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionVerb {
    Copy,
    Cut,
    Paste,
    SelectAll,
}

impl SelectionVerb {
    /// Map from the `jint` Java sends to a verb (unknown → `None`).
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            0 => Some(SelectionVerb::Copy),
            1 => Some(SelectionVerb::Cut),
            2 => Some(SelectionVerb::Paste),
            3 => Some(SelectionVerb::SelectAll),
            _ => None,
        }
    }
}

/// What the app must implement so the system toolbar can drive the editor.
///
/// The app registers one of these (Android-only) in its `android_main`. Each
/// method runs on the GPUI main thread with a `&mut App` (supplied by
/// gpui-android from the focused input handler's context); the app should
/// target the *focused* editor (e.g. via `App::update_window`). Implementations
/// typically reuse the editor's own `copy`/`cut`/`paste`/`select_all` methods so
/// behaviour matches hardware shortcuts.
pub trait SelectionHandler: Send + Sync {
    fn copy(&self, cx: &mut App);
    fn cut(&self, cx: &mut App);
    fn paste(&self, cx: &mut App);
    fn select_all(&self, cx: &mut App);
}

static SELECTION_HANDLER: OnceLock<Arc<dyn SelectionHandler>> = OnceLock::new();

/// Register the app's selection handler. Call once during `android_main`,
/// before the event loop starts.
pub fn set_selection_handler(handler: Arc<dyn SelectionHandler>) {
    let _ = SELECTION_HANDLER.set(handler);
}

/// Invoke the registered handler for a verb, if any handler is set.
fn dispatch_verb(verb: SelectionVerb, cx: &mut App) {
    let Some(handler) = SELECTION_HANDLER.get() else {
        log::warn!("selection: no SelectionHandler registered; ignoring {verb:?}");
        return;
    };
    match verb {
        SelectionVerb::Copy => handler.copy(cx),
        SelectionVerb::Cut => handler.cut(cx),
        SelectionVerb::Paste => handler.paste(cx),
        SelectionVerb::SelectAll => handler.select_all(cx),
    }
}

// ── command queue (Java → Rust, drained on GPUI thread) ─────────────────────

static SELECTION_COMMANDS: parking_lot::Mutex<Vec<SelectionVerb>> = parking_lot::Mutex::new(Vec::new());

/// Enqueue a selection verb from a JNI callback (runs on the Android Binder
/// thread). Safe to call any time; the GPUI thread drains it via
/// [`drain_selection_commands`].
pub fn enqueue_selection_command(verb: SelectionVerb) {
    SELECTION_COMMANDS.lock().push(verb);
}

/// Drain and dispatch any pending selection verbs. Called once per frame from
/// `window.rs`'s `on_request_frame` (already on the GPUI main thread).
///
/// `input_handler` is the focused editor's `PlatformInputHandler`; we borrow
/// its window context (via `update_app`) to obtain a `&mut App` for the app's
/// handler.
pub(crate) fn drain_selection_commands(
    input_handler: &Arc<parking_lot::Mutex<crate::android::window::MainThreadInputHandler>>,
) {
    let mut q = SELECTION_COMMANDS.lock();
    if q.is_empty() {
        return;
    }
    let commands = std::mem::take(&mut *q);
    drop(q);
    let mut guard = input_handler.lock();
    let Some(handler) = guard.inner_mut() else {
        return;
    };
    // 借聚焦 input_handler 的窗口上下文拿到 &mut App，逐个出队执行。
    for verb in commands {
        handler.update_app(|app| dispatch_verb(verb, app));
    }
}

// ── Rust → Java: show / hide the system ActionMode ──────────────────────────

/// Ask Java to start (or refresh) the system `ActionMode` toolbar.
pub fn start_action_mode() {
    let result = with_env(|env| {
        let act = activity(env)?;
        env.call_method(
            &act,
            jni::jni_str!("gpuiStartActionMode"),
            jni::jni_sig!("()V"),
            &[],
        )
        .e()?;
        Ok(())
    });
    if let Err(e) = result {
        log::warn!("start_action_mode failed: {e}");
    }
}

/// Ask Java to dismiss the system `ActionMode` toolbar.
pub fn finish_action_mode() {
    let result = with_env(|env| {
        let act = activity(env)?;
        env.call_method(
            &act,
            jni::jni_str!("gpuiFinishActionMode"),
            jni::jni_sig!("()V"),
            &[],
        )
        .e()?;
        Ok(())
    });
    if let Err(e) = result {
        log::warn!("finish_action_mode failed: {e}");
    }
}
