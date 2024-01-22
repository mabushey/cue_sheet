// cue_sheet
// Copyright (C) 2017  Leonardo Schwarz <mail@leoschwarz.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! Generate a tracklist from a cue file.

// TODO don't swallow errors in parsing but use Result and Option where appropriate.

use errors::Error;
use parser::{self, Command, FileFormat, Time, TrackType};

/// A tracklist provides a more useful representation of the information of a cue sheet.
#[derive(Clone, Debug)]
pub struct Tracklist {
    /// 13 decimal digit UPC/EAN code
    pub catalog: Option<String>,

    /// Files described by the tracklist.
    pub files: Vec<TrackFile>,

    /// Performer of the tracklist.
    pub performer: Option<String>,

    /// Title of the tracklist.
    pub title: Option<String>,

    /// Genre of the tracklist.
    pub genre: Option<String>,

    /// Year of the tracklist.
    pub date: Option<String>,

    /// DiscID of the tracklist.
    pub discid: Option<String>,

    /// Comment of the tracklist.
    // Does this need to be a VEC?
    pub comment: Option<String>,

    /// DiscID of the tracklist.
    pub discnumber: Option<u8>,

    /// DiscID of the tracklist.
    pub totaldiscs: Option<u8>,
}

impl Tracklist {
    /// Parse a cue sheet (content provided as `source`) into a `Tracklist`.
    pub fn parse(source: &str) -> Result<Tracklist, Error> {
        let mut commands = parser::parse_cue(source)?;

        let mut catalog = None;
        let mut performer = None;
        let mut title = None;
        let mut genre = None;
        let mut date = None;
        let mut discid = None;
        let mut comment = None;
        let mut discnumber = None;
        let mut totaldiscs = None;

        while commands.len() > 0 {
            match commands[0].clone() {
                Command::Catalog(p) => {
                    catalog = Some(p);
                    commands.remove(0);
                }
                Command::Performer(p) => {
                    performer = Some(p);
                    commands.remove(0);
                }
                Command::Title(t) => {
                    title = Some(t);
                    commands.remove(0);
                }
                Command::Rem(t, d) => {
                    match t.to_uppercase().as_str() {
                      "GENRE" => genre = Some(d),
                      "DATE" => date = Some(d),
                      "DISCID" => discid = Some(d),
                      "COMMENT" => comment = Some(d),
                      "DISCNUMBER" => {
                        if let Ok(x) = d.parse() {
                          discnumber = Some(x);
                        }
                      },
                      "TOTALDISCS" => {
                        if let Ok(x) = d.parse() {
                          totaldiscs = Some(x);
                        }
                      },
                      _ => (),
                    }
                    commands.remove(0);
                }
                _ => {
                    break;
                }
            }
        }

        let mut files = Vec::new();
        while commands.len() > 0 {
            if let Ok(file) = TrackFile::consume(&mut commands) {
                files.push(file);
            } else {
                break;
            }
        }

        Ok(Tracklist {
            catalog,
            files,
            performer,
            title,
            genre,
            date,
            discid,
            comment,
            discnumber,
            totaldiscs,
        })
    }
}

/// One file described by a tracklist.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackFile {
    /// List of tracks contained in the file.
    pub tracks: Vec<Track>,

    /// The filename.
    pub name: String,

    /// The format of the file.
    pub format: FileFormat,
}

impl TrackFile {
    fn consume(commands: &mut Vec<Command>) -> Result<Self, Error> {
        if let Command::File(name, format) = commands.remove(0) {
            let mut tracks: Vec<Track> = Vec::new();
            let mut last_time: Option<Time> = None;

            while commands.len() > 0 {
                if let Ok(track) = Track::consume(commands) {
                    if track.index.len() > 0 {
                        let time = track.index[track.index.len() - 1].clone();

                        if let Some(start) = last_time {
                            let stop = track.index[0].clone().1;
                            let duration = stop - start;

                            let track_n = tracks.len();
                            if let Some(last_track) = tracks.get_mut(track_n - 1) {
                                (*last_track).duration = Some(duration);
                            }
                        }

                        last_time = Some(time.1);
                    } else {
                        last_time = None;
                    }

                    tracks.push(track);
                } else {
                    break;
                }
            }
            Ok(TrackFile {
                tracks,
                name,
                format,
            })
        } else {
            Err("TrackFile::consume called but no Track command found.".into())
        }
    }
}

/// One track described by a tracklist.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Track {
    /// Title of the track.
    pub title: Option<String>,

    /// Type of the track.
    pub track_type: TrackType,

    /// Duration of the track, if it was possible to determine it.
    ///
    /// This is only possible if tracks have index commands attached to them.
    /// Also note that with just a cue file it is usually not possible to determine the duration of
    /// the last track in the list.
    pub duration: Option<Time>,

    /// Index commands attached to this track (if any).
    pub index: Vec<Index>,

    /// Track number as provided in the cue sheet.
    pub number: u32,

    /// The performer of the track if any was stated.
    pub performer: Option<String>,

    /// International Standard Recording Code of this track
    pub isrc: Option<String>,
}

type Index = (u32, Time);

impl Track {
    fn consume(commands: &mut Vec<Command>) -> Result<Track, Error> {
        if let Command::Track(number, track_type) = commands.remove(0) {
            let mut title = None;
            let mut performer = None;
            let mut isrc = None;
            let mut index = Vec::new();

            while commands.len() > 0 {
                match commands[0].clone() {
                    Command::Performer(p) => {
                        performer = Some(p);
                        commands.remove(0);
                    }
                    Command::Title(t) => {
                        title = Some(t);
                        commands.remove(0);
                    }
                    Command::Isrc(t) => {
                        isrc = Some(t);
                        commands.remove(0);
                    }
                    Command::Pregap(time) => {
                        let next_command = commands
                            .get(1)
                            .ok_or("Pregap is the last command in the track!".to_owned())?
                            .to_owned();

                        let first_index;
                        match next_command {
                            Command::Index(_, time) => first_index = time,
                            _ => {
                                return Err("Pregap is not followed by an index!".into());
                            }
                        }
                        let diff = first_index.total_frames() - time.total_frames();
                        index.push((0, Time::from_frames(diff)));
                        commands.remove(0);
                    }
                    Command::Index(i, time) => {
                        index.push((i, time));
                        commands.remove(0);
                    }
                    _ => break,
                }
            }

            Ok(Track {
                title,
                track_type,
                duration: None,
                index,
                number,
                performer,
                isrc,
            })
        } else {
            Err("Track::consume called but no Track command found.".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample() {
        let source = r#"REM GENRE "Progressive Rock"
REM DATE 1985
REM DISCID DC0E6811
REM COMMENT "ExactAudioCopy v0.95b3"
REM DISCNUMBER 2
REM TOTALDISCS 2
CATALOG 0724349703629
PERFORMER "Marillion"
TITLE "Misplaced Childhood (CD2: Demo)"
FILE "Marillion - Misplaced Childhood (CD2).flac" WAVE
  TRACK 01 AUDIO
    TITLE "Lady Nina"
    PERFORMER "Marillion"
    ISRC GBAYE9801904
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE "Freaks"
    PERFORMER "Marillion"
    ISRC GBAYE9801905
    INDEX 00 05:47:50
    INDEX 01 05:50:10
  TRACK 03 AUDIO
    TITLE "Kayleigh (Alternate Mix)"
    PERFORMER "Marillion"
    ISRC GBAYE9801906
    INDEX 00 09:55:60
    INDEX 01 09:58:20
  TRACK 04 AUDIO
    TITLE "Lavender Blue"
    PERFORMER "Marillion"
    ISRC GBAYE9801907
    INDEX 00 13:57:60
    INDEX 01 14:01:72
  TRACK 05 AUDIO
    TITLE "Heart of Lothian (Extended Mix)"
    PERFORMER "Marillion"
    ISRC GBAYE9801908
    INDEX 00 18:23:15
    INDEX 01 18:24:12
  TRACK 06 AUDIO
    TITLE "Pseudo Silk Kimono (Album Demo)"
    PERFORMER "Marillion"
    ISRC GBAYE9801909
    INDEX 00 24:10:15
    INDEX 01 24:18:17
  TRACK 07 AUDIO
    TITLE "Kayleigh (Album Demo)"
    PERFORMER "Marillion"
    ISRC GBAYE9801910
    INDEX 01 26:29:70
  TRACK 08 AUDIO
    TITLE "Lavender (Album Demo)"
    PERFORMER "Marillion"
    ISRC GBAYE9801911
    INDEX 01 30:36:20
  TRACK 09 AUDIO
    TITLE "Bitter Suite (I. Brief Encounter II. Lost Weekend) (Album Demo)"
    PERFORMER "Marillion"
    ISRC GBAYE9801912
    INDEX 01 33:14:10
    INDEX 02 34:52:55
  TRACK 10 AUDIO
    TITLE "Lords of the Backstage (Album Demo)"
    PERFORMER "Marillion"
    ISRC GBAYE9801913
    INDEX 01 36:08:70
  TRACK 11 AUDIO
    TITLE "Blue Angel (Album Demo)"
    PERFORMER "Marillion"
    ISRC GBAYE9801914
    INDEX 01 37:55:50
  TRACK 12 AUDIO
    TITLE "Misplaced Rendezvous (Album Demo)"
    PERFORMER "Marillion"
    ISRC GBAYE9801915
    INDEX 01 39:42:17
    INDEX 02 41:01:57
  TRACK 13 AUDIO
    TITLE "Heart of Lothian (I. Wide Boy II. Curtain Call) (Album Demo)"
    PERFORMER "Marillion"
    ISRC GBAYE9801916
    INDEX 01 41:38:57
    INDEX 02 44:26:35
  TRACK 14 AUDIO
    TITLE "Waterhole (Expresso Bongo) (Album Demo)"
    PERFORMER "Marillion"
    ISRC GBAYE9801917
    INDEX 00 45:27:70
    INDEX 01 45:28:15
  TRACK 15 AUDIO
    TITLE "Passing Strangers (I. Mylo II. Perimeter Walk III. Threshold) (Album Demo)"
    PERFORMER "Marillion"
    ISRC GBAYE9801918
    INDEX 01 47:28:62
    INDEX 02 49:40:52
    INDEX 03 51:28:62
    INDEX 04 53:45:72
  TRACK 16 AUDIO
    TITLE "Childhoods End? (Album Demo)"
    PERFORMER "Marillion"
    ISRC GBAYE9801919
    INDEX 01 56:45:67
  TRACK 17 AUDIO
    TITLE "White Feather (Album Demo)"
    PERFORMER "Marillion"
    ISRC GBAYE9801920
    INDEX 01 59:09:50"#;

        let tracklist = Tracklist::parse(source).unwrap();
        assert_eq!(tracklist.genre.unwrap(), "Progressive Rock".to_string());
        assert_eq!(tracklist.date.unwrap(), "1985".to_string());
        assert_eq!(tracklist.discid.unwrap(), "DC0E6811".to_string());
        assert_eq!(tracklist.comment.unwrap(), "ExactAudioCopy v0.95b3".to_string());
        assert_eq!(tracklist.discnumber.unwrap(), 2);
        assert_eq!(tracklist.totaldiscs.unwrap(), 2);
        assert_eq!(tracklist.catalog.unwrap(), "0724349703629".to_string());
        assert_eq!(tracklist.performer.unwrap(), "Marillion".to_string());
        assert_eq!(tracklist.title.unwrap(), "Misplaced Childhood (CD2: Demo)".to_string());

        let files = tracklist.files;
        assert_eq!(files.len(), 1);

        let ref f = files[0];
        assert_eq!(f.name, "Marillion - Misplaced Childhood (CD2).flac".to_string());
        assert_eq!(f.format, FileFormat::Wave);

        let ref tracks = f.tracks;
        assert_eq!(tracks.len(), 17);

        assert_eq!(tracks[0].number, 1);
        assert_eq!(tracks[0].track_type, TrackType::Audio);
        assert_eq!(tracks[0].title, Some("Lady Nina".to_string()));
        assert_eq!(tracks[0].performer, Some("Marillion".to_string()));
        assert_eq!(tracks[0].isrc, Some("GBAYE9801904".to_string()));
        //index 1
        assert_eq!(tracks[0].duration, Some(Time::new(5, 47, 50)));

        assert_eq!(tracks[1].number, 2);
        assert_eq!(tracks[1].track_type, TrackType::Audio);
        assert_eq!(tracks[1].title, Some("Freaks".to_string()));
        assert_eq!(tracks[1].performer, Some("Marillion".to_string()));
        assert_eq!(tracks[1].isrc, Some("GBAYE9801905".to_string()));
        //index 0
        //index 1
        //assert_eq!(tracks[0].duration, Some(Time::new(4, 5, 50)));

        assert_eq!(tracks[14].number, 15);
        assert_eq!(tracks[14].track_type, TrackType::Audio);
        assert_eq!(tracks[14].title, Some("Passing Strangers (I. Mylo II. Perimeter Walk III. Threshold) (Album Demo)".to_string()));
        assert_eq!(tracks[14].performer, Some("Marillion".to_string()));
        assert_eq!(tracks[14].isrc, Some("GBAYE9801918".to_string()));
        //index 1
        //index 2
        //index 3
        //index 4
        //assert_eq!(tracks[0].duration, Some(Time::new(9, 17, 5)));

        assert_eq!(tracks[15].number, 16);
        assert_eq!(tracks[15].track_type, TrackType::Audio);
        assert_eq!(tracks[15].title, Some("Childhoods End? (Album Demo)".to_string()));
        assert_eq!(tracks[15].performer, Some("Marillion".to_string()));
        assert_eq!(tracks[15].isrc, Some("GBAYE9801919".to_string()));
        //index 1
        //assert_eq!(tracks[0].duration, Some(Time::new(2, 28, 63)));
    }

    #[test]
    fn pregap() {
        let src = r#"FILE "disc.img" BINARY
                       TRACK 01 MODE1/2352
                         INDEX 01 00:00:00
                       TRACK 02 AUDIO
                         PREGAP 00:02:00
                         INDEX 01 58:41:36
                       TRACK 03 AUDIO
                         INDEX 00 61:06:08
                         INDEX 01 61:08:08"#;

        let tracklist = Tracklist::parse(src).unwrap();

        let ref f = tracklist.files[0];
        let ref tracks = f.tracks;

        assert_eq!(tracks[0].index[0], (1, Time::new(0, 0, 0)));
        assert_eq!(tracks[1].index[0], (0, Time::new(58, 39, 36)));
        assert_eq!(tracks[1].index[1], (1, Time::new(58, 41, 36)));
        assert_eq!(tracks[2].index[0], (0, Time::new(61, 06, 08)));
        assert_eq!(tracks[2].index[1], (1, Time::new(61, 08, 08)));
    }
}
