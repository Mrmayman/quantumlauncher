// QuantumLauncher TUI - Authentication Event Handler

use crate::tui::{app::App, AuthEvent};

/// Handle authentication events from async operations
pub fn handle_auth_event(app: &mut App, event: AuthEvent) {
    // Delegate to the App's built-in handler to avoid cross-module access to private methods
    app.handle_auth_event(event);
}
