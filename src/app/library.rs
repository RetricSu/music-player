//use id3::Tag;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
//use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    root_path: PathBuf,
    items: Vec<LibraryItem>,
}

impl Library {
    pub fn new(root_path: PathBuf) -> Self {
        Self {
            root_path,
            items: Vec::new(),
        }
    }

    pub fn root_path(&self) -> PathBuf {
        self.root_path.clone()
    }

    pub fn items(&self) -> Vec<LibraryItem> {
        self.items.clone()
    }

    pub fn add_item(&mut self, library_item: LibraryItem) {
        self.items.push(library_item);
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct LibraryItem {
    path: PathBuf,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    year: Option<i32>,
    genre: Option<String>,
    track_number: Option<u32>,
}

impl LibraryItem {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            title: None,
            artist: None,
            album: None,
            year: None,
            genre: None,
            track_number: None,
        }
    }

    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn set_title(&mut self, title: Option<&str>) -> Self {
        if let Some(title) = title {
            self.title = Some(title.to_string());
        }

        self.to_owned()
    }

    pub fn title(&self) -> Option<String> {
        self.title.clone()
    }

    pub fn set_artist(&mut self, artist: Option<&str>) -> Self {
        if let Some(artist) = artist {
            self.artist = Some(artist.to_string());
        }
        self.to_owned()
    }

    pub fn artist(&self) -> Option<String> {
        self.artist.clone()
    }

    pub fn set_album(&mut self, album: Option<&str>) -> Self {
        if let Some(album) = album {
            self.album = Some(album.to_string());
        }
        self.to_owned()
    }

    pub fn album(&self) -> Option<String> {
        self.album.clone()
    }

    pub fn set_year(&mut self, year: Option<i32>) -> Self {
        self.year = year;
        self.to_owned()
    }

    pub fn year(&self) -> Option<i32> {
        self.year.clone()
    }

    pub fn set_genre(&mut self, genre: Option<&str>) -> Self {
        if let Some(genre) = genre {
            self.genre = Some(genre.to_string());
        }
        self.to_owned()
    }

    pub fn genre(&self) -> Option<String> {
        self.genre.clone()
    }

    pub fn set_track_number(&mut self, track_number: Option<u32>) -> Self {
        self.track_number = track_number;
        self.to_owned()
    }

    pub fn track_number(&self) -> Option<u32> {
        self.track_number.clone()
    }
}
