use crate::config::PanelVisibility;
use crate::theme::Theme;
use std::io;

pub(crate) fn is_supported_platform() -> bool {
    cfg!(any(target_os = "linux", target_os = "macos"))
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn run(
    initial_theme: Option<Theme>,
    hidden_agents: &[String],
    panels: PanelVisibility,
    demo_mode: bool,
) -> io::Result<()> {
    supported::run(initial_theme, hidden_agents, panels, demo_mode)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn run(
    _initial_theme: Option<Theme>,
    _hidden_agents: &[String],
    _panels: PanelVisibility,
    _demo_mode: bool,
) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "--float is supported only on Linux and macOS",
    ))
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod supported {
    use super::*;
    use crate::app::App;
    use crate::demo;
    use crate::model::{AgentSession, SessionStatus};
    use eframe::egui::{self, Color32, RichText};
    use std::time::{Duration, Instant};

    const REFRESH_INTERVAL: Duration = Duration::from_secs(2);

    pub(super) fn run(
        initial_theme: Option<Theme>,
        hidden_agents: &[String],
        panels: PanelVisibility,
        demo_mode: bool,
    ) -> io::Result<()> {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("abtop")
                .with_inner_size([460.0, 360.0])
                .with_min_inner_size([360.0, 180.0])
                .with_always_on_top(),
            ..Default::default()
        };
        let hidden_agents = hidden_agents.to_vec();
        eframe::run_native(
            "abtop",
            options,
            Box::new(move |_cc| {
                Ok(Box::new(FloatingApp::new(
                    initial_theme,
                    hidden_agents,
                    panels,
                    demo_mode,
                )))
            }),
        )
        .map_err(|err| io::Error::other(err.to_string()))
    }

    struct FloatingApp {
        app: App,
        demo_mode: bool,
        last_tick: Instant,
    }

    impl FloatingApp {
        fn new(
            initial_theme: Option<Theme>,
            hidden_agents: Vec<String>,
            panels: PanelVisibility,
            demo_mode: bool,
        ) -> Self {
            let mut app =
                App::new_with_config(initial_theme.unwrap_or_default(), &hidden_agents, panels);
            if demo_mode {
                demo::populate_demo(&mut app);
            } else {
                app.tick();
            }
            Self {
                app,
                demo_mode,
                last_tick: Instant::now(),
            }
        }

        fn refresh_if_needed(&mut self) {
            if self.demo_mode || self.last_tick.elapsed() < REFRESH_INTERVAL {
                return;
            }
            self.app.tick();
            self.last_tick = Instant::now();
        }
    }

    impl eframe::App for FloatingApp {
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            self.refresh_if_needed();
            ctx.request_repaint_after(Duration::from_millis(500));

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.visuals_mut().override_text_color = Some(Color32::from_rgb(230, 236, 242));
                ui.heading("abtop");
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("floating task status")
                            .color(Color32::from_rgb(150, 164, 180)),
                    );
                    ui.separator();
                    ui.label(format!("{} sessions", self.app.sessions.len()));
                });
                ui.add_space(8.0);

                if self.app.sessions.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new("no sessions").color(Color32::from_rgb(150, 164, 180)),
                        );
                    });
                    return;
                }

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for session in &self.app.sessions {
                            render_session(ui, session, &self.app);
                            ui.add_space(6.0);
                        }
                    });
            });
        }
    }

    fn render_session(ui: &mut egui::Ui, session: &AgentSession, app: &App) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("●").color(status_color(&session.status)));
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(session_label(session, app)).strong());
                        ui.label(
                            RichText::new(status_label(&session.status))
                                .color(status_color(&session.status)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("{:.0}% ctx", session.context_percent));
                        });
                    });
                    ui.label(
                        RichText::new(task_label(session)).color(Color32::from_rgb(202, 211, 222)),
                    );
                });
            });
        });
    }

    fn session_label(session: &AgentSession, app: &App) -> String {
        let title = app
            .summaries
            .get(&session.session_id)
            .map(String::as_str)
            .filter(|summary| !summary.is_empty())
            .unwrap_or(&session.project_name);
        truncate_chars(title, 32)
    }

    fn task_label(session: &AgentSession) -> String {
        session
            .current_tasks
            .last()
            .map(String::as_str)
            .filter(|task| !task.is_empty())
            .map(|task| truncate_chars(task, 72))
            .unwrap_or_else(|| "—".to_string())
    }

    fn status_label(status: &SessionStatus) -> &'static str {
        match status {
            SessionStatus::Thinking => "Thinking",
            SessionStatus::Executing => "Executing",
            SessionStatus::Waiting => "Waiting",
            SessionStatus::RateLimited => "RateLimited",
            SessionStatus::Done => "Done",
        }
    }

    fn status_color(status: &SessionStatus) -> Color32 {
        match status {
            SessionStatus::Thinking => Color32::from_rgb(245, 190, 80),
            SessionStatus::Executing => Color32::from_rgb(84, 200, 120),
            SessionStatus::Waiting => Color32::from_rgb(100, 170, 255),
            SessionStatus::RateLimited => Color32::from_rgb(240, 90, 90),
            SessionStatus::Done => Color32::from_rgb(130, 140, 150),
        }
    }

    fn truncate_chars(value: &str, max: usize) -> String {
        if value.chars().count() <= max {
            value.to_string()
        } else {
            let mut truncated: String = value.chars().take(max.saturating_sub(1)).collect();
            truncated.push('…');
            truncated
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_supported_platform;

    #[test]
    fn platform_support_matches_declared_scope() {
        assert_eq!(
            is_supported_platform(),
            cfg!(any(target_os = "linux", target_os = "macos"))
        );
    }
}
