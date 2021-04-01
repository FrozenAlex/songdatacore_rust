use crate::beatstar::data::{BeatStarDataFile, BeatStarSong, BeatStarSongJson, RustCStringWrapper};
use crate::beatstar::BEAT_STAR_FILE;
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;
use std::time::Duration;
use stopwatch::Stopwatch;
use ureq::{Agent, Response};

extern crate chrono;

pub const SCRAPED_SCORE_SABER_URL: &str = "https://raw.githubusercontent.com/andruzzzhka/BeatSaberScrappedData/master/combinedScrappedData.json";

const HTTP_OK: u16 = 200;

lazy_static! {
    static ref AGENT: Agent = ureq::AgentBuilder::new()
        .timeout_read(Duration::from_secs(5))
        .timeout_write(Duration::from_secs(5))
        .build();
}

///
/// Returns None unless error, in which you get the response
///
/// Fetches the latest song data and stores it indefinitely
///
pub fn beatstar_update_database() -> Option<Response> {
    if BEAT_STAR_FILE.get().is_none() {
        println!("Fetching from internet");
        let mut stopwatch = Stopwatch::start_new();
        let response = AGENT.get(SCRAPED_SCORE_SABER_URL).call().unwrap();
        println!(
            "Received data from internet in {0}ms",
            stopwatch.elapsed().as_millis()
        );

        if response.status() == HTTP_OK {
            let body: Vec<BeatStarSongJson> = response.into_json().unwrap();
            println!(
                "Parsed beat file into json data in {0}ms",
                stopwatch.elapsed().as_millis()
            );

            let parsed_data = parse_beatstar(&body);

            BEAT_STAR_FILE.get_or_init(|| parsed_data);

            println!(
                "Fully parsed beat file in {0}ms",
                stopwatch.elapsed().as_millis()
            );

            stopwatch.stop();
        } else {
            return Some(response);
        }
    }

    None
}

///
/// Get the song list and clone it
///
#[no_mangle]
pub extern "C" fn beatstar_retrieve_database_extern() -> *const BeatStarDataFile {
    match beatstar_retrieve_database() {
        Ok(e) => e,
        Err(e) => panic!(
            "Unable to fetch from database {0}",
            e.into_string().unwrap()
        ),
    }
}

///
/// Get the song list and clone it
///
pub fn beatstar_retrieve_database() -> Result<&'static BeatStarDataFile, Response> {
    if let Some(e) = beatstar_update_database() {
        return Err(e);
    }

    let bsf_mutex = BEAT_STAR_FILE.get().unwrap();

    Ok(bsf_mutex)
}

///
/// Get the song based on hash
///
///
#[no_mangle]
pub unsafe extern "C" fn beatstar_get_song_extern(hash: *const c_char) -> *const BeatStarSong {
    if hash.is_null() {
        return ptr::null_mut();
    }

    let raw = CStr::from_ptr(hash);

    let hash_str = match raw.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match beatstar_get_song(hash_str) {
        Ok(e) => match e {
            None => ptr::null(),
            Some(e) => e,
        },
        Err(e) => panic!(
            "Unable to fetch from database {0}",
            e.into_string().unwrap()
        ),
    }
}

///
/// Gets a song based on it's hash
///
pub fn beatstar_get_song(hash: &str) -> Result<Option<&BeatStarSong>, Response> {
    return match beatstar_update_database() {
        None => Ok(BEAT_STAR_FILE
            .get()
            .unwrap()
            .songs
            .get(&RustCStringWrapper::new(hash.into()))),
        Some(e) => Err(e),
    };
}

///
/// Parses the entire JSON
/// This takes an average of 700 MS, do better?
///
fn parse_beatstar(songs: &[BeatStarSongJson]) -> BeatStarDataFile {
    let mut song_converted: Vec<BeatStarSong> = vec![];

    for song in songs {
        song_converted.push(BeatStarSong::convert(&song))
    }

    let mut song_map: HashMap<RustCStringWrapper, BeatStarSong> = HashMap::new();

    for mut song in song_converted {
        song.characteristics = HashMap::new();

        for diff in &mut song.diffs {
            let diff_type = diff.get_diff_type();

            let map = song
                .characteristics
                .entry(diff_type)
                .or_insert_with(HashMap::new);

            map.insert(diff.diff.clone(), diff.clone());
        }

        song_map.insert(song.hash.clone(), song);
    }

    BeatStarDataFile { songs: song_map }
}
