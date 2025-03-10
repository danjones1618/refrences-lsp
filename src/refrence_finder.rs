use log::info;
use regex::Regex;
use std::{collections::HashMap, fs, time::SystemTime};

use lsp_types::{Position, Range};

/// A character range within a single line.
/// The same as a `lsP_types::Range` with `Range.start.line == Range.end.line`
#[derive(Clone, Debug)]
pub struct InlineRange {
    line: u32,
    start_character: u32,
    end_character: u32,
}

#[derive(Clone)]
pub enum InFileRefrenceType {
    JiraRefrence { ticket: String },
    GitHubUrlRefrence { url: String },
    GitLabUrlRefrence { url: String },
}

pub struct InFileRefrence {
    pub marker: InFileRefrenceType,
    pub range: InlineRange,
}

struct CachedFileRefrence {
    last_modified_time: SystemTime,
    refrences: Vec<InFileRefrence>,
}

pub struct RefrenceFinder {
    file_refrences_map: HashMap<String, CachedFileRefrence>,
    refrence_regex: Regex,
}

impl RefrenceFinder {
    pub fn new() -> RefrenceFinder {
        RefrenceFinder {
            file_refrences_map: HashMap::new(),
            refrence_regex: Regex::new(r"(?<jira_ticket>[A-Z]{3,}-\d+)|(?<new_line>\n)").unwrap(),
        }
    }
    // |ABC-123\n

    pub fn get_refrences<'a>(
        &'a mut self,
        file_path: &'a str,
    ) -> impl Iterator<Item = &InFileRefrence> {
        if !self.file_refrences_map.contains_key(file_path) {
            self.find_refrences_in_file(file_path);
        }
        let cached_refrences = self.file_refrences_map.get(file_path).unwrap();
        let last_modified_time = fs::metadata(file_path)
            .expect("uh oh todo file path errors")
            .modified()
            .expect("todo handle error");
        if cached_refrences.last_modified_time < last_modified_time {
            self.find_refrences_in_file(file_path);
        }
        let cached_refrences = self.file_refrences_map.get(file_path).unwrap();
        cached_refrences.refrences.iter()
    }

    fn find_refrences_in_file(&mut self, file_path: &str) {
        info!("Analysing refrences for {file_path}");
        let mut current_line = 0;
        let mut line_start_position = 0;
        let file_contents = fs::read_to_string(file_path).expect("TODO: wrong file path handling");
        let last_modified_time = fs::metadata(file_path)
            .expect("uh oh todo file path errors")
            .modified()
            .expect("todo handle error");

        let refrences = self
            .refrence_regex
            .captures_iter(&file_contents)
            .filter_map(|found_match| {
                if let Some(found_match) = found_match.name("new_line") {
                    current_line += 1;
                    line_start_position = found_match.start();
                    return None;
                }
                if let Some(found_match) = found_match.name("jira_ticket") {
                    return Some(InFileRefrence {
                        marker: InFileRefrenceType::JiraRefrence {
                            ticket: found_match.as_str().to_owned(),
                        },
                        range: InlineRange {
                            line: current_line,
                            start_character: (found_match.start() - line_start_position - 1) as u32,
                            end_character: (found_match.end() - line_start_position) as u32,
                        },
                    });
                }
                panic!("Missing regex capture group");
            })
            .collect();
        self.file_refrences_map.insert(
            file_path.to_owned(),
            CachedFileRefrence {
                refrences,
                last_modified_time,
            },
        );
    }
}

impl InlineRange {
    pub fn contains_position(&self, other_position: Position) -> bool {
        self.line == other_position.line
            && self.start_character <= other_position.character
            && other_position.character < self.end_character
    }

    pub fn start_position(&self) -> Position {
        Position {
            line: self.line,
            character: self.start_character,
        }
    }

    pub fn end_position(&self) -> Position {
        Position {
            line: self.line,
            character: self.end_character,
        }
    }
}

pub enum InlineRangeTryFromError {
    MultiLineRange,
}

impl TryFrom<Range> for InlineRange {
    type Error = InlineRangeTryFromError;

    fn try_from(value: Range) -> Result<Self, Self::Error> {
        if value.start.line != value.end.line {
            return Err(InlineRangeTryFromError::MultiLineRange);
        }
        Ok(InlineRange {
            line: value.start.line,
            start_character: value.start.character,
            end_character: value.end.character,
        })
    }
}

impl Into<Range> for InlineRange {
    fn into(self) -> Range {
        Range {
            start: Position {
                line: self.line,
                character: self.start_character,
            },
            end: Position {
                line: self.line,
                character: self.end_character,
            },
        }
    }
}
