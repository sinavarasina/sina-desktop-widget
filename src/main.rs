//! Native WayWi desktop widget suite (`sina-desktop-widget`).

mod app;
mod modules;
mod theme;
mod widgets;

#[cfg(test)]
mod tests;

use waywi::prelude::*;

use crate::app::ipc::CONTROL_SOCKET_NAME;

struct SinaDesktopWidget;

impl Application for SinaDesktopWidget {
    type State = app::State;
    type Message = app::Msg;

    fn init(ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) -> Result<Self::State> {
        app::handlers::init_state(ctx)
    }

    fn update(
        state: &mut Self::State,
        msg: Self::Message,
        ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
    ) -> Result<()> {
        app::handlers::update_state(state, msg, ctx)
    }

    fn view(state: &Self::State) -> Surfaces<Self::Message> {
        modules::build_surfaces(state)
    }

    fn subscriptions(state: &Self::State) -> Subscriptions<Self::Message> {
        app::handlers::build_subscriptions(state)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        let first = args[0].as_str();
        if matches!(first, "help" | "-h" | "--help") {
            println!("{}", app::ipc::CLI_HELP_TEXT);
            return;
        }
        if matches!(first, "version" | "-V" | "--version") {
            println!("sina-desktop-widget {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        let cmd = args.join(" ");
        if let Err(err) = waywi::control::send_command(CONTROL_SOCKET_NAME, &cmd) {
            eprintln!("sina-desktop-widget control error: {err}");
            std::process::exit(1);
        }
        return;
    }

    let startup_cfg =
        app::config::WidgetConfig::load_from_path(&app::config::WidgetConfig::default_path());
    let renderer_pref = std::env::var("SINA_RENDERER")
        .ok()
        .and_then(|v| app::config::RendererMode::parse(&v))
        .unwrap_or(startup_cfg.general.renderer)
        .to_preference();
    let memory_cfg = std::env::var("SINA_MEMORY")
        .ok()
        .and_then(|v| app::config::MemoryProfile::parse(&v))
        .unwrap_or(startup_cfg.general.memory_profile)
        .to_memory_config();

    if let Err(err) = App::new()
        .name("sina-desktop-widget")
        .renderer(renderer_pref)
        .memory(memory_cfg)
        .run::<SinaDesktopWidget>()
    {
        eprintln!("sina-desktop-widget error: {err}");
        std::process::exit(1);
    }
}
