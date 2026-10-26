use super::App;
use eframe::egui;
use std::time::Duration;
mod performance;
pub(in crate::app) use performance::Performance;
impl App {
    pub(super) fn snapshot(&mut self, ctx: &egui::Context) {
        if let Some((path, launched, requested)) = &mut self.smoke {
            if !*requested
                && launched.elapsed() > Duration::from_secs(2)
                && !self.checking
                && !self.audio.is_refreshing()
            {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                *requested = true;
            }
            let snapshot = ctx.input(|input| {
                input.events.iter().find_map(|event| {
                    if let egui::Event::Screenshot { image, .. } = event {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(snapshot) = snapshot {
                let bytes: Vec<u8> = snapshot
                    .pixels
                    .iter()
                    .flat_map(|pixel| pixel.to_array())
                    .collect();
                image::save_buffer(
                    path,
                    &bytes,
                    snapshot.size[0] as u32,
                    snapshot.size[1] as u32,
                    image::ColorType::Rgba8,
                )
                .expect("save UI smoke snapshot");
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}
