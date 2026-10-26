use eframe::egui::{Context, ViewportCommand, ViewportId};

/// Apply window changes explicitly to the main viewport, never the picker.
pub(super) fn minimize_main(ctx: &Context) {
    ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Minimized(true));
}

pub(super) fn restore_main(ctx: &Context) {
    ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Visible(true));
    ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Focus);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn minimize_and_restore_target_the_main_window() {
        let ctx = Context::default();
        let out = ctx.run(Default::default(), minimize_main);
        assert!(
            out.viewport_output[&ViewportId::ROOT]
                .commands
                .iter()
                .any(|c| matches!(c, ViewportCommand::Minimized(true)))
        );
        let out = ctx.run(Default::default(), restore_main);
        let commands = &out.viewport_output[&ViewportId::ROOT].commands;
        assert!(
            commands
                .iter()
                .any(|c| matches!(c, ViewportCommand::Minimized(false)))
        );
        assert!(
            commands
                .iter()
                .any(|c| matches!(c, ViewportCommand::Visible(true)))
        );
        assert!(commands.iter().any(|c| matches!(c, ViewportCommand::Focus)));
    }
}
