use crate::{Answer, Exercise, ExerciseKind};
use std::collections::HashSet;
use unicode_normalization::UnicodeNormalization;

/// Normalize presentation only. Accents, internal apostrophes and lexical
/// differences remain significant. No unrestricted fuzzy matching.
pub fn normalize(value: &str) -> String {
    value
        .nfc()
        .collect::<String>()
        .replace(['\u{2019}', '\u{2018}'], "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end_matches(|c: char| c.is_whitespace() || ['.', '!', '?', '…'].contains(&c))
        .to_lowercase()
}

pub fn answer_text(exercise: &Exercise, answer: &Answer) -> Result<String, String> {
    match (&exercise.kind, answer) {
        (ExerciseKind::Choice | ExerciseKind::Reading, Answer::Option(id)) => exercise
            .options
            .iter()
            .find(|option| &option.id == id)
            .map(|option| option.text.clone())
            .ok_or_else(|| "Choose an available answer.".into()),
        (ExerciseKind::Ordering, Answer::Tiles(ids)) => {
            let unique: HashSet<_> = ids.iter().collect();
            if unique.len() != ids.len() || ids.is_empty() {
                return Err("Use each tile at most once.".into());
            }
            ids.iter()
                .map(|id| {
                    exercise
                        .options
                        .iter()
                        .find(|option| &option.id == id)
                        .map(|option| option.text.clone())
                        .ok_or_else(|| "Unknown word tile.".into())
                })
                .collect::<Result<Vec<_>, String>>()
                .map(|words| words.join(" "))
        }
        (ExerciseKind::Translation | ExerciseKind::Cloze, Answer::Text(value))
            if !value.trim().is_empty() && value.len() <= 4000 =>
        {
            Ok(value.clone())
        }
        _ => Err("Provide an answer in the expected format.".into()),
    }
}

pub fn evaluate(exercise: &Exercise, answer: &Answer) -> Result<bool, String> {
    let normalized = normalize(&answer_text(exercise, answer)?);
    Ok(exercise
        .accepted
        .iter()
        .any(|expected| normalize(expected) == normalized))
}
