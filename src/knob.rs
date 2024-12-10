use std::f32::consts::PI;

use bevy_egui::egui::epaint::{ColorImage, Rect, TextureHandle, Vec2};
use bevy_egui::egui::{self, Id, Image, ImageSource, Sense, Widget};

pub struct Knob<'a> {
    value: &'a mut f32,
    knob_image: Box<dyn Fn() -> ColorImage>,
    scale_image: Box<dyn Fn() -> ColorImage>,
}

impl<'a> Knob<'a> {
    pub fn new(
        value: &'a mut f32,
        knob_image: impl Fn() -> ColorImage + 'static,
        scale_image: impl Fn() -> ColorImage + 'static,
    ) -> Self {
        Knob {
            value,
            knob_image: Box::new(knob_image),
            scale_image: Box::new(scale_image),
        }
    }

    fn get_tex(
        ui: &mut egui::Ui,
        mem_id: &str,
        constructor: impl Fn() -> ColorImage,
    ) -> TextureHandle {
        let id = Id::new(mem_id);

        let has_tex = ui.memory_mut(|mem| mem.data.get_temp::<TextureHandle>(id).is_some());

        if has_tex {
            ui.memory_mut(|mem| mem.data.get_temp::<TextureHandle>(id).unwrap())
        } else {
            let handle = ui
                .ctx()
                .load_texture("knob", constructor(), Default::default());

            ui.memory_mut(|mem| mem.data.insert_temp(id, handle.clone()));

            handle
        }
    }
}

impl<'a> Widget for Knob<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let scale_img = Image::new(ImageSource::from(&Self::get_tex(
            ui,
            "scale-tex",
            &self.scale_image,
        )))
        .fit_to_exact_size(Vec2::splat(76.0));
        let scale_resp = ui.add(scale_img);
        let scale_rect = scale_resp.rect;

        const OFFSET: f32 = (15.0 / 360.0) * (2.0 * PI);
        let angle = *self.value * (2.0 * PI - 2.0 * OFFSET) + OFFSET;
        let knob_img = Image::new(ImageSource::from(&Self::get_tex(
            ui,
            "knob-tex",
            &self.knob_image,
        )))
        .fit_to_exact_size(Vec2::splat(50.0))
        .rotate(angle, Vec2::splat(0.5))
        .sense(Sense::hover());

        let mut resp = ui.put(
            Rect::from_center_size(scale_rect.center(), knob_img.size().unwrap()),
            knob_img,
        );

        if resp.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll != 0.0 {
                *self.value += scroll / 360.0;
                *self.value = self.value.clamp(0.0, 1.0);
                resp.mark_changed();
            }
        }

        resp
    }
}