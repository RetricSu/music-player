use crate::app::library::LibraryItem;
use crate::app::playlist::Playlist;
use crate::AudioCommand;
use std::sync::mpsc::Sender;

pub struct Player {
    pub track_state: TrackState,
    pub selected_track: Option<LibraryItem>,
    pub audio_tx: Sender<AudioCommand>,
    pub volume: f32,
    pub seek_in_seconds: u32,
}

impl Player {
    pub fn new(audio_cmd_tx: Sender<AudioCommand>) -> Self {
        Self {
            track_state: TrackState::Unstarted,
            selected_track: None,
            audio_tx: audio_cmd_tx,
            volume: 1.0,
            seek_in_seconds: 0, // TODO: This should have subsecond precision, but is okay for now.
        }
    }

    pub fn is_stopped(&self) -> bool {
        match self.track_state {
            TrackState::Stopped => true,
            _ => false,
        }
    }

    pub fn seek_to(&mut self, seconds: u32) {
        self.seek_in_seconds = seconds;
        self.audio_tx
            .send(AudioCommand::Seek(seconds))
            .expect("Failed to send seek to audio thread");
    }

    // TODO: Should return Result
    pub fn stop(&mut self) {
        match &self.track_state {
            TrackState::Playing | TrackState::Paused => {
                self.track_state = TrackState::Stopped;
                self.audio_tx
                    .send(AudioCommand::Stop)
                    .expect("Failed to send stop to audio thread");
                //self.sink.stop();
            }
            _ => (),
        }
    }

    // TODO: Should return Result
    pub fn play(&mut self) {
        if let Some(selected_track) = &self.selected_track {
            /*
            let file = std::io::BufReader::new(
                std::fs::File::open(&selected_track.path()).expect("Failed to open file"),
            );
            */
            //let source = rodio::Decoder::new(file).expect("Failed to decode audio file");

            match self.track_state {
                TrackState::Unstarted | TrackState::Stopped | TrackState::Playing => {
                    self.track_state = TrackState::Playing;
                    let track_path = selected_track.path();
                    self.audio_tx
                        .send(AudioCommand::LoadFile(track_path))
                        .expect("Failed to send to audio thread");

                    /*
                    let sink_try = rodio::Sink::try_new(&self.stream_handle);

                    match sink_try {
                        Ok(sink) => {
                            self.sink = sink;
                            self.sink.append(source);
                        }
                        Err(e) => tracing::error!("{:?}", e),
                    }
                    */
                }
                TrackState::Paused => {
                    self.track_state = TrackState::Playing;
                    self.audio_tx
                        .send(AudioCommand::Play)
                        .expect("Failed to send play to audio thread");
                    //self.sink.play();
                }
            }
        }
    }

    // TODO: Should return result
    pub fn pause(&mut self) {
        match self.track_state {
            TrackState::Playing => {
                self.track_state = TrackState::Paused;
                self.audio_tx
                    .send(AudioCommand::Pause)
                    .expect("Failed to send pause to audio thread");
                //self.sink.pause();
            }
            TrackState::Paused => {
                self.track_state = TrackState::Playing;
                self.audio_tx
                    .send(AudioCommand::Play)
                    .expect("Failed to send play to audio thread");
                //self.sink.play();
            }
            _ => (),
        }
    }

    pub fn previous(&mut self, playlist: &Playlist) {
        if let Some(selected_track) = &self.selected_track {
            if let Some(current_track_position) = playlist.get_pos(&selected_track) {
                if current_track_position > 0 {
                    let previous_track = &playlist.tracks[current_track_position - 1];
                    self.selected_track = Some(previous_track.clone());
                    self.play();
                }
            }
        }
    }

    pub fn next(&mut self, playlist: &Playlist) {
        if let Some(selected_track) = &self.selected_track {
            if let Some(current_track_position) = playlist.get_pos(&selected_track) {
                if current_track_position < playlist.tracks.len() - 1 {
                    let next_track = &playlist.tracks[current_track_position + 1];
                    self.selected_track = Some(next_track.clone());
                    self.play();
                }
            }
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume;
        //self.sink.set_volume(volume);
    }

    pub fn set_seek_in_seconds(&mut self, seek_in_seconds: u32) {
        self.seek_in_seconds = seek_in_seconds;
    }
}

pub enum TrackState {
    Unstarted,
    Stopped,
    Playing,
    Paused,
}

impl std::fmt::Display for TrackState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TrackState::Unstarted => write!(f, "Unstarted"),
            TrackState::Stopped => write!(f, "Stopped"),
            TrackState::Playing => write!(f, "Playing"),
            TrackState::Paused => write!(f, "Paused"),
        }
    }
}
