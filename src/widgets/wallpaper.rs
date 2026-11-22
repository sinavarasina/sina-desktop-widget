//! Embedded Wayland `Layer::Background` wallpaper surface with zero-retention SHM and
//! reclaimable decoded pixels (`ReleaseAfterCommit` + `ReclaimAfterPresent`).

use waywi::prelude::*;

use crate::app::config::WallpaperFit;
use crate::app::state::Msg;
use crate::theme::CRUST;

pub fn build_wallpaper_surface(asset: ImageAsset, fit: WallpaperFit) -> Surface<Msg> {
    Surface::memo("desktop-wallpaper", &(asset, fit), |&(img, fit_mode)| {
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
                    .fit(fit_mode.to_image_fit())
                    .filter(ImageFilter::Bilinear),
            )
    })
}
