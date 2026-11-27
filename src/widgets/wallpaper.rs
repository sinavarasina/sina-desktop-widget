//! Embedded Wayland `Layer::Background` wallpaper surface with zero-retention SHM and
//! reclaimable decoded pixels (`ReleaseAfterCommit` + `ReclaimAfterPresent`).

use waywi::prelude::*;

use crate::app::config::WallpaperFit;
use crate::app::state::Msg;
use crate::theme::CRUST;

pub fn build_wallpaper_surface(
    asset: ImageAsset,
    fit: WallpaperFit,
    width: f32,
    height: f32,
) -> Surface<Msg> {
    let key = (asset, fit, width.to_bits(), height.to_bits());
    Surface::memo("desktop-wallpaper", &key, |&(img, fit_mode, wb, hb)| {
        let w = f32::from_bits(wb).max(320.0);
        let h = f32::from_bits(hb).max(240.0);
        Surface::overlay("desktop-wallpaper", Output::All)
            .layer(Layer::Background)
            .ignore_exclusive_zone()
            .anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT)
            .size(Size::new(w.logical_px(), h.logical_px()))
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
