//! Native WayWi desktop widget suite (`sina-desktop-widget`).

mod app;
mod modules;
mod theme;
mod widgets;

#[cfg(test)]
mod tests;

use waywi::prelude::*;

struct SinaDesktopWidget;

impl Application for SinaDesktopWidget {
    type State = app::State;
    type Message = app::Msg;

    fn init(ctx: &mut EffectCtx<'_>) -> Result<Self::State> {
        app::handlers::init_state(ctx)
    }

    fn update(state: &mut Self::State, msg: Self::Message, ctx: &mut EffectCtx<'_>) -> Result<()> {
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
    if let Err(err) = App::new()
        .name("sina-desktop-widget")
        .renderer(RendererPreference::Software)
        .run::<SinaDesktopWidget>()
    {
        eprintln!("sina-desktop-widget error: {err}");
        std::process::exit(1);
    }
}
