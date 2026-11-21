//! Embedded Wayland `Layer::Background` wallpaper surface with zero-retention SHM and
//! reclaimable decoded pixels (`ReleaseAfterCommit` + `ReclaimAfterPresent`).

use waywi::prelude::*;

use crate::app::state::Msg;
use crate::theme::CRUST;

pub fn build_wallpaper_surface(asset: ImageAsset) -> Surface<Msg> {
    Surface::memo("desktop-wallpaper", &asset, |&img| {
        Surface::overlay("desktop-wallpaper", Output::All)
            .layer(Layer::Background)
            .ignore_exclusive_zone()
            .anchor(Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right)
            .size(Size::fill())
            .input_mode(InputMode::PassThrough)
            .keyboard_interactivity(KeyboardInteractivity::None)
            .buffer_retention(BufferRetention::ReleaseAfterCommit)
            .background(CRUST)
            .child(
                Image::new(img)
                    .size(Size::fill())
                    .fit(ImageFit::Cover)
                    .filter(ImageFilter::Bilinear),
            )
    })
}
