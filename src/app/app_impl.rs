use eframe::egui;

use super::{App, LibraryCommand};
use crate::app::components::{
    footer::Footer, library_component::LibraryComponent, menu_bar::MenuBar,
    player_component::PlayerComponent, playlist_table::PlaylistTable, playlist_tabs::PlaylistTabs,
    scope_component::ScopeComponent, AppComponent,
};

impl eframe::App for App {
    fn on_exit(&mut self, _ctx: Option<&eframe::glow::Context>) {
        tracing::info!("exiting and saving");
        self.save_state();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        ctx.request_repaint();

        if let Some(lib_cmd_rx) = &self.library_cmd_rx {
            if let Ok(lib_cmd) = lib_cmd_rx.try_recv() {
                match lib_cmd {
                    LibraryCommand::AddItem(lib_item) => self.library.add_item(lib_item),
                    LibraryCommand::AddView(lib_view) => self.library.add_view(lib_view),
                    LibraryCommand::AddPathId(path_id) => {
                        self.library.set_path_to_imported(path_id)
                    }
                }
            }
        }

        if let Some(selected_track) = &self.player.as_mut().unwrap().selected_track {
            let display = format!(
                "{} - {} [ Music Player ]",
                &selected_track
                    .artist()
                    .unwrap_or("unknown artist".to_string()),
                &selected_track
                    .title()
                    .unwrap_or("unknown title".to_string())
            );

            ctx.send_viewport_cmd(egui::ViewportCommand::Title(display));
        }

        egui::TopBottomPanel::top("MusicPlayer").show(ctx, |ui| {
            MenuBar::add(self, ui);
        });

        egui::TopBottomPanel::top("Player").show(ctx, |ui| {
            PlayerComponent::add(self, ui);
            ScopeComponent::add(self, ui);
        });

        egui::TopBottomPanel::bottom("Footer").show(ctx, |ui| {
            Footer::add(self, ui);
        });

        egui::CentralPanel::default().show(ctx, |_ui| {
            egui::SidePanel::left("Library Window")
                .default_width(350.0)
                .show(ctx, |ui| {
                    LibraryComponent::add(self, ui);
                });
        });

        egui::CentralPanel::default().show(ctx, |_ui| {
            egui::TopBottomPanel::top("Playlist Tabs").show(ctx, |ui| {
                PlaylistTabs::add(self, ui);
            });

            egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(_current_playlist_idx) = &mut self.current_playlist_idx {
                    egui::ScrollArea::both().show(ui, |ui| {
                        PlaylistTable::add(self, ui);
                    });
                }
            });
        });
    }
}
