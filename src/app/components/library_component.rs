use super::AppComponent;
use crate::app::App;

pub struct LibraryComponent;

impl AppComponent for LibraryComponent {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        eframe::egui::ScrollArea::both().show(ui, |ui| {
            eframe::egui::CollapsingHeader::new(eframe::egui::RichText::new("All Music"))
                .default_open(true)
                .show(ui, |ui| {
                    for container in &ctx.library.view().containers {
                        let items = &container.items;
                        // todo: correct the name to remove this patch
                        let album_name = if container.name.is_empty() || container.name == "<?>" {
                            "unknown album".to_string()
                        } else {
                            container.name.clone()
                        };

                        let library_group = eframe::egui::CollapsingHeader::new(
                            eframe::egui::RichText::new(album_name),
                        )
                        .default_open(false)
                        .show(ui, |ui: &mut eframe::egui::Ui| {
                            for item in &container.items {
                                let item_label = ui.add(
                                    eframe::egui::Label::new(eframe::egui::RichText::new(
                                        item.title().unwrap_or("unknown title".to_string()),
                                    ))
                                    .sense(eframe::egui::Sense::click()),
                                );

                                if item_label.double_clicked() {
                                    if let Some(current_playlist_idx) = &ctx.current_playlist_idx {
                                        let current_playlist =
                                            &mut ctx.playlists[*current_playlist_idx];

                                        current_playlist.add(item.clone());
                                    }
                                }
                            }
                        });

                        if let Some(current_playlist_idx) = &ctx.current_playlist_idx {
                            let current_playlist = &mut ctx.playlists[*current_playlist_idx];

                            if library_group.header_response.double_clicked() {
                                for item in items {
                                    current_playlist.add(item.clone());
                                }
                            }
                        }
                    }
                });
        });
    }
}
