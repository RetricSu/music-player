pub use crate::app::player::Player;
pub use crate::app::App;
pub use crate::app::*;

use std::path::PathBuf;
use std::sync::atomic::AtomicU32;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::thread;

use eframe::egui;
use symphonia::core::codecs::{DecoderOptions, FinalizeResult, CODEC_TYPE_NULL};
use symphonia::core::errors::{Error, Result};
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, Track};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

mod app;
mod output;
mod resampler;

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("App booting...");

    let (tx, rx) = channel();
    let (audio_tx, audio_rx) = channel();
    let (ui_tx, ui_rx) = channel();
    let cursor = Arc::new(AtomicU32::new(0));
    let player = Player::new(audio_tx, ui_rx, cursor);

    // App setup
    let mut app = App::load().unwrap_or_default();
    app.player = Some(player);
    app.library_sender = Some(tx);
    app.library_receiver = Some(rx);

    // Audio output setup
    let _audio_thread = thread::spawn(move || {
        let mut state = PlayerState::Unstarted;

        let mut audio_engine_state = AudioEngineState {
            reader: None,
            audio_output: None,
            track_num: None,
            seek: None,
            decode_opts: None,
            track_info: None,
        };

        let mut decoder: Option<Box<dyn symphonia::core::codecs::Decoder>> = None;
        let mut _volume = 1.0;
        let mut current_track_path: Option<PathBuf> = None;
        let time_base = 1.0 / 44100.0; // This needs to be based on the file...
        // let mut current_track_seconds = 0.0;

        loop {
            process_audio_cmd(&audio_rx, &mut state);

            match state {
                PlayerState::Playing => {
                    // decode the next packet.

                    let result = loop {
                        process_audio_cmd(&audio_rx, &mut state);

                        if state != PlayerState::Playing {
                            break Ok(())
                        }  
                        
                        let reader = audio_engine_state.reader.as_mut().unwrap();
                        let play_opts = audio_engine_state.track_info.unwrap();
                        let audio_output = &mut audio_engine_state.audio_output;
                        // Get the next packet from the format reader.
                        let packet = match reader.next_packet() {
                            Ok(packet) => packet,
                            Err(err) =>  {
                                tracing::warn!("couldn't decode next packet");
                                break Err(err)
                            },
                        };


                        let current_track_seconds = *&packet.ts as f64 * time_base;
                        ui_tx
                            .send(UiCommand::CurrentSeconds(current_track_seconds as u64))
                            .expect("Failed to send play to audio thread");

                        // If the packet does not belong to the selected track, skip it.
                        if packet.track_id() != play_opts.track_id {
                            tracing::warn!("packet track id doesn't match track id");
                            continue;
                        }
                    
                        // Decode the packet into audio samples.
                        match decoder.as_mut().unwrap().decode(&packet) {
                            Ok(decoded) => {
                                // If the audio output is not open, try to open it.
                                if audio_output.is_none() {
                                    // Get the audio buffer specification. This is a description of the decoded
                                    // audio buffer's sample format and sample rate.
                                    let spec = *decoded.spec();
                
                                    // Get the capacity of the decoded buffer. Note that this is capacity, not
                                    // length! The capacity of the decoded buffer is constant for the life of the
                                    // decoder, but the length is not.
                                    let duration = decoded.capacity() as u64;
                
                                    // Try to open the audio output.
                                    audio_output.replace(output::try_open(spec, duration).unwrap());
                                }
                                else {
                                    // TODO: Check the audio spec. and duration hasn't changed.
                                }
                
                                // Write the decoded audio samples to the audio output if the presentation timestamp
                                // for the packet is >= the seeked position (0 if not seeking).
                                if packet.ts() >= play_opts.seek_ts {
                
                                    // TODO - Send the progress back to GUI
                                    // if !no_progress {
                                    //     print_progress(packet.ts(), dur, tb);
                                    // }
                
                                    if let Some(audio_output) = audio_output {
                                        audio_output.write(decoded).unwrap()
                                    }
                                }
                            }
                            Err(Error::DecodeError(err)) => {
                                // Decode errors are not fatal. Print the error message and try to decode the next
                                // packet as usual.
                                tracing::warn!("decode error: {}", err);
                            }
                            Err(err) => break Err(err),
                        }
                    };

                    if result.is_err() {
                        tracing::error!("playing error");
                    }

                    // Return if a fatal error occured.
                    ignore_end_of_stream_error(result).expect("failed to ignore EoF");
                
                    // Finalize the decoder and return the verification result if it's been enabled.
                    _ = do_verification(decoder.as_mut().unwrap().finalize());
                },
                PlayerState::Stopped => {
                    // Flush the audio buffer and reset the cpal audio context, which gets reconfigured on the next file loaded.
                    if let Some(audio_output) = audio_engine_state.audio_output.as_mut() {
                        audio_output.flush()
                    }

                    audio_engine_state.audio_output = None;
                },
                PlayerState::SeekTo(seconds) => {
                    if let Some(ref current_track_path) = current_track_path {
                        // Stop current playback
                        if let Some(audio_output) = audio_engine_state.audio_output.as_mut() {
                            audio_output.flush()
                        }
                        
                        audio_engine_state.audio_output = None;

                        load_file(current_track_path, &mut audio_engine_state, &mut decoder, seconds as f64);
                        state = PlayerState::Playing;
                    }
                },
                PlayerState::LoadFile(ref path) => {
                    // Stop current playback
                    if let Some(audio_output) = audio_engine_state.audio_output.as_mut() {
                        audio_output.flush()
                    }
                    
                    audio_engine_state.audio_output = None;
                    
                    current_track_path = Some((*path).clone());
                    load_file(path, &mut audio_engine_state, &mut decoder, 0.0);
                    // TODO - Get total u64 track duration and send to Ui
                    // ui_tx
                    //     .send(UiCommand::TotalTrackDuration(current_track_seconds as u64))
                    //     .expect("Failed to send play to audio thread");

                    state = PlayerState::Playing;
                }
                PlayerState::Paused => {
                    // don't decode AND don't flush the buffer?
                },
                PlayerState::Unstarted => {},
            }
        }       
    }); // Audio Thread end

    let mut window_options = eframe::NativeOptions::default();
    window_options.initial_window_size = Some(egui::Vec2::new(1024., 768.));
    eframe::run_native("Music Player", window_options, Box::new(|_| Box::new(app)))
        .expect("eframe failed: I should change main to return a result and use anyhow");
}


fn process_audio_cmd(audio_rx: &Receiver<AudioCommand>, state: &mut PlayerState) {
    match audio_rx.try_recv() {
        Ok(cmd) => {
            //Process Start
            match cmd {
                AudioCommand::Seek(seconds) => {
                    tracing::info!("Processing SEEK command for {} seconds", seconds);
                    *state = PlayerState::SeekTo(seconds);
                }
                AudioCommand::Stop => {
                    tracing::info!("Processing STOP command");
                    *state = PlayerState::Stopped;
                }
                AudioCommand::Pause => {
                    tracing::info!("Processing PAUSE command");
                    *state = PlayerState::Paused;
                }
                AudioCommand::Play => {
                    tracing::info!("Processing PLAY command");
                    *state = PlayerState::Playing;
                }
                AudioCommand::LoadFile(path) => {
                    tracing::info!("Processing LOAD FILE command for path: {:?}", &path);   
                    *state = PlayerState::LoadFile(path);                             
                }
                _ => tracing::warn!("Unhandled case in audio command loop"),
            }
        },
        Err(_) => (),   // When no commands are sent, this will evaluate. aka - it is the
                        // common case. No need to print anything
    }   
}


enum SeekPosition {
    Time(f64),
    Timetamp(u64),
}

#[derive(Copy, Clone)]
struct PlayTrackOptions {
    track_id: u32,
    seek_ts: u64,
}


#[derive(PartialEq)]
pub enum PlayerState {
    Unstarted,
    Stopped,
    Playing,
    Paused,
    LoadFile(PathBuf),
    SeekTo(u64),
}

struct AudioEngineState {
    pub reader: Option<Box<dyn FormatReader>>,
    pub audio_output: Option<Box<dyn output::AudioOutput>>,
    pub track_num: Option<usize>,
    pub seek: Option<SeekPosition>,
    pub decode_opts: Option<DecoderOptions>,
    pub track_info: Option<PlayTrackOptions>,
}

fn load_file(
    path: &PathBuf, 
    audio_engine_state: &mut AudioEngineState, 
    decoder: &mut Option<Box<dyn symphonia::core::codecs::Decoder>>, 
    seek_to_seconds: f64
) {
    let hint = Hint::new();
    let source = Box::new(std::fs::File::open(path).expect("couldn't open file"));
    let mss = MediaSourceStream::new(source, Default::default());
    let format_opts = FormatOptions { enable_gapless: true, ..Default::default() };
    let metadata_opts: MetadataOptions = Default::default();
    let seek = Some(SeekPosition::Time(seek_to_seconds));

    match symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts) {
        Ok(probed) => {
            // Set the decoder options.
            let decode_opts = DecoderOptions { verify: true, ..Default::default() };

            audio_engine_state.reader = Some(probed.format);
            audio_engine_state.decode_opts = Some(decode_opts);
            audio_engine_state.seek = seek;                                
            
            // Configure everything for playback.
            _ = setup_audio_reader(audio_engine_state);

            let reader = audio_engine_state.reader.as_mut().unwrap();
            let play_opts = audio_engine_state.track_info.unwrap();
            let decode_opts = audio_engine_state.decode_opts.unwrap();                                
        
            let track = match reader.tracks().iter().find(|track| track.id == play_opts.track_id) {
                Some(track) => track,
                _ => {
                    tracing::warn!("Couldn't find track");
                    return ();
                }
            };
        
            // Create a decoder for the track.
            *decoder = Some(symphonia::default::get_codecs().make(&track.codec_params, &decode_opts).expect("Failed to get decoder"));
        
            // Get the selected track's timebase and duration.
            let _tb = track.codec_params.time_base;
            let _dur = track.codec_params.n_frames.map(|frames| track.codec_params.start_ts + frames);

            tracing::info!("Track Duration: {}, TimeBase: {}", _dur.unwrap_or(0), _tb.unwrap());
        }
        Err(err) => {
            // The input was not supported by any format reader.
            tracing::warn!("the audio format is not supported: {}", err);
            // Err(err);
        }
    }
}


fn setup_audio_reader(audio_engine_state: &mut AudioEngineState) -> Result<i32>  {
    // If the user provided a track number, select that track if it exists, otherwise, select the
    // first track with a known codec.
    let reader = audio_engine_state.reader.as_mut().unwrap();
    let seek = &audio_engine_state.seek;

    let track = audio_engine_state.track_num
        .and_then(|t| reader.tracks().get(t))
        .or_else(|| first_supported_track(reader.tracks()));

    let mut track_id = match track {
        Some(track) => track.id,
        _ => return Ok(0),
    };

    // If seeking, seek the reader to the time or timestamp specified and get the timestamp of the
    // seeked position. All packets with a timestamp < the seeked position will not be played.
    //
    // Note: This is a half-baked approach to seeking! After seeking the reader, packets should be
    // decoded and *samples* discarded up-to the exact *sample* indicated by required_ts. The
    // current approach will discard excess samples if seeking to a sample within a packet.
    let seek_ts = if let Some(seek) = seek {
        let seek_to = match seek {
            SeekPosition::Time(t) => SeekTo::Time { time: Time::from(*t), track_id: Some(track_id) },
            SeekPosition::Timetamp(ts) => SeekTo::TimeStamp { ts: *ts, track_id },
        };

        // Attempt the seek. If the seek fails, ignore the error and return a seek timestamp of 0 so
        // that no samples are trimmed.
        match reader.seek(SeekMode::Accurate, seek_to) {
            Ok(seeked_to) => seeked_to.required_ts,
            Err(Error::ResetRequired) => {
                tracing::warn!("reset required...");
                // print_tracks(reader.tracks());
                track_id = first_supported_track(reader.tracks()).unwrap().id;
                0
            }
            Err(err) => {
                // Don't give-up on a seek error.
                tracing::warn!("seek error: {}", err);
                0
            }
        }
    }
    else {
        // If not seeking, the seek timestamp is 0.
        0
    };

    tracing::info!("seek ts: {}", seek_ts);

    audio_engine_state.track_info = Some(PlayTrackOptions { track_id, seek_ts });

   Ok(0)
}

fn first_supported_track(tracks: &[Track]) -> Option<&Track> {
    tracks.iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
}

fn ignore_end_of_stream_error(result: Result<()>) -> Result<()> {
    match result {
        Err(Error::IoError(err))
            if err.kind() == std::io::ErrorKind::UnexpectedEof
                && err.to_string() == "end of stream" =>
        {
            // Do not treat "end of stream" as a fatal error. It's the currently only way a
            // format reader can indicate the media is complete.
            Ok(())
        }
        _ => result,
    }
}

fn do_verification(finalization: FinalizeResult) -> Result<i32> {
    match finalization.verify_ok {
        Some(is_ok) => {
            // Got a verification result.
            tracing::info!("verification: {}", if is_ok { "passed" } else { "failed" });

            Ok(i32::from(!is_ok))
        }
        // Verification not enabled by user, or unsupported by the codec.
        _ => Ok(0),
    }
}

