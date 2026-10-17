use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub level: String,
    pub time_zone: String,
    pub daily_goal: u32,
    pub theme: String,
    pub reduced_motion: bool,
    pub text_scale: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExerciseKind {
    Choice,
    Translation,
    Ordering,
    Cloze,
    Reading,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OptionItem {
    pub id: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Exercise {
    pub id: String,
    pub kind: ExerciseKind,
    pub instruction: String,
    pub prompt: String,
    pub context: Option<String>,
    pub options: Vec<OptionItem>,
    pub accepted: Vec<String>,
    pub hint: String,
    pub explanation: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Lesson {
    pub id: String,
    pub version: u32,
    pub level: String,
    pub title: String,
    pub objective: String,
    pub editorial_status: String,
    pub license: String,
    pub exercises: Vec<Exercise>,
}

impl Lesson {
    pub fn summary(&self) -> LessonSummary {
        LessonSummary {
            id: self.id.clone(),
            level: self.level.clone(),
            title: self.title.clone(),
            objective: self.objective.clone(),
            editorial_status: self.editorial_status.clone(),
            exercise_count: self.exercises.len(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonSummary {
    pub id: String,
    pub level: String,
    pub title: String,
    pub objective: String,
    pub editorial_status: String,
    pub exercise_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Answer {
    Text(String),
    Option(String),
    Tiles(Vec<String>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Feedback {
    pub correct: bool,
    pub revealed: bool,
    pub expected: String,
    pub explanation: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExerciseView {
    pub id: String,
    pub kind: ExerciseKind,
    pub instruction: String,
    pub prompt: String,
    pub context: Option<String>,
    pub options: Vec<OptionItem>,
    pub hint: Option<String>,
    pub draft: Option<Answer>,
    pub feedback: Option<Feedback>,
    pub ready_to_continue: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub id: String,
    pub lesson: LessonSummary,
    pub position: usize,
    pub status: String,
    pub exercise: Option<ExerciseView>,
    pub first_correct: u32,
    pub independent_correct: u32,
    pub hinted_items: u32,
    pub revealed_items: u32,
    pub xp_earned: u32,
    pub practice_only: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyDay {
    pub date: String,
    pub xp: u32,
    pub goal: u32,
    pub time_zone: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub total_xp: u32,
    pub today_xp: u32,
    pub completed_lessons: u32,
    pub current_streak: u32,
    pub best_streak: u32,
    pub achievements: Vec<String>,
    pub days: Vec<StudyDay>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub settings: Option<Settings>,
    pub lessons: Vec<LessonSummary>,
    pub active_session: Option<SessionView>,
    pub progress: Progress,
    pub development_build: bool,
    pub content_issues: Vec<String>,
}
