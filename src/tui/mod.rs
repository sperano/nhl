// Module declarations
pub mod components;
pub mod widgets;

// Core modules
pub mod action;
pub mod component;
pub mod component_store;
pub mod constants;
pub mod document;
pub mod document_nav;
pub mod effects;
pub mod helpers;
pub mod keys;
pub mod nav_handler;
pub mod reducer;
pub mod reducers;
pub mod renderer;
pub mod runtime;
pub mod settings_helpers;
pub mod state;
pub mod tab_component;
pub mod table;
pub mod types;

pub mod testing;

#[cfg(test)]
mod integration_tests;

pub use action::Action;
pub use component::{Component, Effect, Element, ElementWidget};
pub use document::DocumentRenderCache;
pub use effects::DataEffects;
pub use keys::key_to_action;
pub use reducer::reduce;
pub use renderer::Renderer;
pub use runtime::Runtime;
pub use state::AppState;
pub use table::{Alignment, CellValue, ColumnDef};
pub use types::{SettingsCategory, StackedDocument, Tab};

use crate::config::Config;
use crate::data_provider::NHLDataProvider;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// While idle (nothing loading, no animation), redraw at most this often so the status bar's
/// "Updated Ns ago" text stays live without a full rebuild+render every poll cycle.
const IDLE_REDRAW_INTERVAL: Duration = Duration::from_secs(1);

/// Check if an action is a quit action
fn is_quit_action(action: &Action) -> bool {
    matches!(action, Action::Quit)
}

/// Find the next available screenshot counter by scanning existing files
#[cfg(feature = "development")]
fn get_next_screenshot_counter() -> u32 {
    use std::fs;

    let mut max_counter = 0;

    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.flatten() {
            if let Some(filename) = entry.file_name().to_str() {
                // Match pattern: nhl-screenshot-NNN.txt where NNN is 3 digits
                if filename.starts_with("nhl-screenshot-") && filename.ends_with(".txt") {
                    // Extract the part between "nhl-screenshot-" and ".txt"
                    if let Some(middle) = filename
                        .strip_prefix("nhl-screenshot-")
                        .and_then(|s| s.strip_suffix(".txt"))
                    {
                        // Only accept if it's exactly 3 digits
                        if middle.len() == 3 && middle.chars().all(|c| c.is_ascii_digit()) {
                            if let Ok(counter) = middle.parse::<u32>() {
                                max_counter = max_counter.max(counter);
                            }
                        }
                    }
                }
            }
        }
    }

    max_counter + 1
}

/// Enables raw mode and switches to the alternate screen, returning a ready-to-use terminal.
fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

/// Leaves the alternate screen, disables mouse capture, and restores normal terminal mode.
fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), io::Error> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()
}

/// Whether the loading spinner should keep animating: true while any data fetch is in
/// flight, or while core data (standings, schedule) or any open document's associated
/// data hasn't loaded yet.
fn needs_animation(state: &AppState) -> bool {
    !state.data.loading.is_empty()
        || state.data.standings.is_none()
        || state.data.schedule.is_none()
        || state
            .navigation
            .document_stack
            .iter()
            .any(|doc| match &doc.document {
                StackedDocument::Boxscore { game_id, .. } => {
                    state.data.boxscores.get(game_id).is_none()
                }
                StackedDocument::TeamDetail { abbrev, season } => season.is_none_or(|s| {
                    !state
                        .data
                        .team_roster_stats
                        .contains_key(&(abbrev.clone(), s))
                }),
                StackedDocument::PlayerDetail { player_id, .. } => {
                    state.data.player_data.get(player_id).is_none()
                }
            })
}

/// Saves a just-captured screenshot buffer to disk and dispatches a status message
/// action reporting success or failure.
#[cfg(feature = "development")]
fn save_screenshot_and_notify(runtime: &mut Runtime, buffer: ratatui::buffer::Buffer) {
    let counter = get_next_screenshot_counter();
    let filename = format!("nhl-screenshot-{:03}.txt", counter);
    let area = ratatui::layout::Rect::new(0, 0, buffer.area().width, buffer.area().height);
    if let Err(e) = crate::dev::screenshot::save_buffer_screenshot(&buffer, area, &filename) {
        tracing::error!("Failed to save screenshot: {}", e);
        runtime.dispatch(Action::SetStatusMessage {
            message: format!("Failed to save screenshot: {}", e),
            is_error: true,
        });
    } else {
        tracing::info!("Screenshot saved to {}", filename);
        runtime.dispatch(Action::SetStatusMessage {
            message: format!("Screenshot saved: {}", filename),
            is_error: false,
        });
    }
}

/// Renders the current virtual tree to the terminal. In development builds, if a
/// screenshot was requested, captures the freshly-rendered buffer and saves it via
/// `save_screenshot_and_notify`.
///
/// Returns `true` when a screenshot was just saved, so the caller can force one more
/// dirty render pass to display the resulting status message.
fn render_frame(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    runtime: &mut Runtime,
    doc_cache: &std::cell::RefCell<DocumentRenderCache>,
    #[cfg(feature = "development")] screenshot_requested: &mut bool,
) -> Result<bool, io::Error> {
    #[cfg(feature = "development")]
    let mut screenshot_buffer: Option<ratatui::buffer::Buffer> = None;

    terminal.draw(|f| {
        let area = f.area();

        // Set global background color if theme specifies one
        let theme = &runtime.state().system.config.display.theme;
        if let Some(bg_color) = theme.as_ref().and_then(|t| t.bg) {
            f.buffer_mut()
                .set_style(area, ratatui::style::Style::default().bg(bg_color));
        }

        // Build virtual tree from current state
        // This creates component states if they don't exist yet
        let element = runtime.build();

        // Update viewport heights for document-based components
        // Called after build() to ensure component states exist
        runtime.update_viewport_heights(area.height);

        // Render virtual tree to ratatui buffer
        let config = &runtime.state().system.config.display;
        let ctx = crate::config::RenderContext::focused(config).with_doc_cache(doc_cache);
        let mut renderer = Renderer::new();
        renderer.render(element, area, f.buffer_mut(), &ctx);

        // Clone buffer if screenshot requested
        #[cfg(feature = "development")]
        if *screenshot_requested {
            screenshot_buffer = Some(f.buffer_mut().clone());
        }
    })?;

    #[cfg(feature = "development")]
    if let Some(buffer) = screenshot_buffer {
        *screenshot_requested = false;
        save_screenshot_and_notify(runtime, buffer);
        return Ok(true);
    }

    Ok(false)
}

/// Polls the runtime for actions completed by background effects and detects terminal
/// resizes, marking `dirty` when either occurs. Returns the number of actions processed,
/// which the caller uses to decide whether to loop again immediately.
fn sync_runtime_state(
    runtime: &mut Runtime,
    terminal: &Terminal<CrosstermBackend<io::Stdout>>,
    dirty: &mut bool,
) -> Result<usize, io::Error> {
    // Process any actions from effects FIRST (so data loads trigger re-render)
    let actions_processed = runtime.process_actions();
    if actions_processed > 0 {
        tracing::debug!("LOOP: Processed {} actions", actions_processed);
        *dirty = true;
    }

    // Detect terminal resize without requiring a draw call, so resize is caught even
    // when nothing else would otherwise make this iteration dirty.
    let term_size = terminal.size()?;
    if runtime.state().system.terminal_width != term_size.width {
        runtime.dispatch(Action::UpdateTerminalWidth(term_size.width));
        *dirty = true;
    }

    Ok(actions_processed)
}

/// If `dirty`, renders the current frame and updates render bookkeeping. In development
/// builds, a just-completed screenshot save forces another dirty pass so its status
/// message gets displayed.
fn maybe_render_frame(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    runtime: &mut Runtime,
    doc_cache: &std::cell::RefCell<DocumentRenderCache>,
    dirty: &mut bool,
    last_render_at: &mut Instant,
    #[cfg(feature = "development")] screenshot_requested: &mut bool,
) -> Result<(), io::Error> {
    if !*dirty {
        return Ok(());
    }

    #[cfg(feature = "development")]
    let screenshot_saved = render_frame(terminal, runtime, doc_cache, screenshot_requested)?;
    #[cfg(not(feature = "development"))]
    render_frame(terminal, runtime, doc_cache)?;

    *dirty = false;
    *last_render_at = Instant::now();

    #[cfg(feature = "development")]
    if screenshot_saved {
        *dirty = true; // status message above needs to be shown
    }

    Ok(())
}

/// Checks whether the loading animation needs to keep running, dispatches `Tick` (which
/// also drives periodic auto-refresh -- see `reducer::reduce`), and marks `dirty` if the
/// animation needs to advance or the idle redraw interval has elapsed.
///
/// `Tick` is dispatched unconditionally: skipping it while idle would silently stop
/// auto-refresh once initial data loads. Tick alone does NOT mark the frame dirty (it
/// fires every poll cycle and would defeat the point of the dirty flag).
///
/// Returns whether animation is in progress, which the caller uses to pick a faster poll
/// timeout for smoother animation.
fn tick(runtime: &mut Runtime, dirty: &mut bool, last_render_at: Instant) -> bool {
    let should_animate = needs_animation(runtime.state());

    runtime.dispatch(Action::Tick);
    if should_animate || last_render_at.elapsed() >= IDLE_REDRAW_INTERVAL {
        *dirty = true;
    }

    should_animate
}

/// Converts a key event to an action (if any) and dispatches it through the runtime.
///
/// Returns `(dispatched, should_quit)`: whether an action was actually dispatched, and
/// whether that action was `Action::Quit`. Marks `dirty` when an action is dispatched so
/// the caller knows to force a re-render.
fn dispatch_key_action(runtime: &mut Runtime, key: KeyEvent, dirty: &mut bool) -> (bool, bool) {
    let action = key_to_action(key, runtime.state(), runtime.component_states());
    let should_quit = action.as_ref().is_some_and(is_quit_action);

    if let Some(act) = action {
        runtime.dispatch(act);
        *dirty = true;
        (true, should_quit)
    } else {
        (false, should_quit)
    }
}

/// Builds the initial `Runtime` (state, `DataEffects` handler) and kicks off the
/// first data load.
fn init_runtime(client: Arc<dyn NHLDataProvider>, config: Config) -> Runtime {
    let data_effects = Arc::new(DataEffects::new(client));

    let mut initial_state = AppState::default();
    initial_state.system.config = config.clone();
    initial_state.system.reset_status_message();

    let mut runtime = Runtime::new(initial_state, data_effects);
    runtime.dispatch(Action::RefreshData);
    runtime
}

/// Outcome of polling for and handling a single keyboard event.
enum KeyPollOutcome {
    /// No key event arrived, or one was handled without quitting: keep looping.
    Continue,
    /// `Action::Quit` was dispatched: the event loop should exit.
    Quit,
}

/// Polls for a keyboard event within `poll_timeout_ms` and, if one arrives, handles it:
/// in development builds, Shift-S is intercepted to request a screenshot; otherwise the
/// key is converted to an action and dispatched via `dispatch_key_action`.
fn poll_and_handle_key(
    runtime: &mut Runtime,
    poll_timeout_ms: u64,
    dirty: &mut bool,
    #[cfg(feature = "development")] screenshot_requested: &mut bool,
) -> Result<KeyPollOutcome, io::Error> {
    if !event::poll(Duration::from_millis(poll_timeout_ms))? {
        return Ok(KeyPollOutcome::Continue);
    }
    let Event::Key(key) = event::read()? else {
        return Ok(KeyPollOutcome::Continue);
    };

    #[cfg(feature = "development")]
    {
        use crossterm::event::{KeyCode, KeyModifiers};
        if key.code == KeyCode::Char('S') && key.modifiers.contains(KeyModifiers::SHIFT) {
            tracing::info!("Screenshot requested via Shift-S");
            *screenshot_requested = true;
            return Ok(KeyPollOutcome::Continue);
        }
    }

    // Convert key to action, dispatch it, and check for quit
    let (dispatched, should_quit) = dispatch_key_action(runtime, key, dirty);

    if dispatched && !should_quit {
        tracing::debug!("ACTION: Continuing loop for immediate re-render");
    }

    if should_quit {
        tracing::debug!("ACTION: Quitting application");
        return Ok(KeyPollOutcome::Quit);
    }

    Ok(KeyPollOutcome::Continue)
}

/// Forces a dirty pass when a screenshot is pending, then renders the frame if dirty.
fn render_step(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    runtime: &mut Runtime,
    doc_cache: &std::cell::RefCell<DocumentRenderCache>,
    dirty: &mut bool,
    last_render_at: &mut Instant,
    #[cfg(feature = "development")] screenshot_requested: &mut bool,
) -> Result<(), io::Error> {
    #[cfg(feature = "development")]
    {
        if *screenshot_requested {
            *dirty = true; // force a draw so we have a buffer to capture
        }
        maybe_render_frame(
            terminal,
            runtime,
            doc_cache,
            dirty,
            last_render_at,
            screenshot_requested,
        )
    }
    #[cfg(not(feature = "development"))]
    maybe_render_frame(terminal, runtime, doc_cache, dirty, last_render_at)
}

/// Advances the loading animation via `tick`, then polls for and handles a keyboard
/// event with a timeout chosen for smoother animation while it's running.
fn animate_and_poll_key(
    runtime: &mut Runtime,
    dirty: &mut bool,
    last_render_at: Instant,
    #[cfg(feature = "development")] screenshot_requested: &mut bool,
) -> Result<KeyPollOutcome, io::Error> {
    let should_animate = tick(runtime, dirty, last_render_at);

    // Poll for keyboard events - use shorter timeout when animating for smoother animation
    let poll_timeout = if should_animate { 50 } else { 100 };
    #[cfg(feature = "development")]
    return poll_and_handle_key(runtime, poll_timeout, dirty, screenshot_requested);
    #[cfg(not(feature = "development"))]
    poll_and_handle_key(runtime, poll_timeout, dirty)
}

/// Runs the main TUI loop until the user quits: syncs runtime state from completed
/// effects, renders when dirty, advances the loading animation, and polls for and
/// handles keyboard input.
fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    runtime: &mut Runtime,
    doc_cache: &std::cell::RefCell<DocumentRenderCache>,
) -> Result<(), io::Error> {
    #[cfg(feature = "development")]
    let mut screenshot_requested = false;

    // Tracks whether the next loop iteration needs a real rebuild+render. Skipping the
    // draw call when nothing changed is what keeps the app from burning CPU at ~10Hz while
    // completely idle; every place that mutates rendered-visible state must set this.
    let mut dirty = true; // must render once at startup
    let mut last_render_at = Instant::now();

    loop {
        let actions_processed = sync_runtime_state(runtime, terminal, &mut dirty)?;

        #[cfg(feature = "development")]
        render_step(
            terminal,
            runtime,
            doc_cache,
            &mut dirty,
            &mut last_render_at,
            &mut screenshot_requested,
        )?;
        #[cfg(not(feature = "development"))]
        render_step(
            terminal,
            runtime,
            doc_cache,
            &mut dirty,
            &mut last_render_at,
        )?;

        // If actions were processed, continue loop immediately to check for more
        // This ensures UI updates immediately when async data arrives
        if actions_processed > 0 {
            tracing::debug!(
                "Processed {} actions, continuing loop immediately for re-render",
                actions_processed
            );
            continue;
        }

        #[cfg(feature = "development")]
        let outcome = animate_and_poll_key(
            runtime,
            &mut dirty,
            last_render_at,
            &mut screenshot_requested,
        )?;
        #[cfg(not(feature = "development"))]
        let outcome = animate_and_poll_key(runtime, &mut dirty, last_render_at)?;

        if matches!(outcome, KeyPollOutcome::Quit) {
            break;
        }
    }

    Ok(())
}

/// Main entry point for TUI mode
pub async fn run(client: Arc<dyn NHLDataProvider>, config: Config) -> Result<(), io::Error> {
    let mut terminal = setup_terminal()?;
    let mut runtime = init_runtime(client, config);

    // Cross-frame document render cache: reuses each document's full-height
    // render while its inputs are unchanged, so scrolling and the idle 1Hz
    // status-bar redraw don't rebuild documents. See DocumentRenderCache.
    let doc_cache = std::cell::RefCell::new(DocumentRenderCache::default());

    event_loop(&mut terminal, &mut runtime, &doc_cache)?;

    restore_terminal(&mut terminal)?;

    Ok(())
}

#[cfg(test)]
mod tests;
