use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Serialize, FromRow)]
pub(crate) struct Lesson {
    pub id: String,
    pub timetable_entry_id: Option<String>,
    pub teacher_id: String,
    pub classroom_id: String,
    pub class_name: String,
    pub subject_id: String,
    pub subject_name: String,
    pub lesson_date: String,
    pub starts_at: String,
    pub ends_at: String,
    pub room: String,
    pub topic: String,
    pub learning_goals: String,
    pub status: String,
    pub pupil_count: i64,
    pub attendance_count: i64,
    pub present_count: i64,
    pub performance_count: i64,
}

#[derive(Deserialize)]
pub(crate) struct LessonInput {
    pub classroom_id: String,
    pub subject_id: String,
    pub lesson_date: String,
    pub starts_at: String,
    pub ends_at: String,
    #[serde(default)]
    pub room: String,
    #[serde(default)]
    pub topic: String,
    #[serde(default)]
    pub learning_goals: String,
    #[serde(default)]
    pub repeat_until: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct LessonEdit {
    pub lesson_date: String,
    pub starts_at: String,
    pub ends_at: String,
    #[serde(default)]
    pub room: String,
    #[serde(default)]
    pub topic: String,
    #[serde(default)]
    pub learning_goals: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, FromRow)]
pub(crate) struct Performance {
    pub student_id: String,
    pub rating: i64,
    #[serde(default)]
    pub observation: String,
}

#[derive(Serialize, FromRow)]
pub(crate) struct ClassTask {
    pub id: String,
    pub classroom_id: String,
    pub class_name: String,
    pub lesson_id: Option<String>,
    pub title: String,
    pub description: String,
    pub completion_mode: String,
    pub due_date: Option<String>,
    pub completed_at: Option<i64>,
    pub assigned_count: i64,
    pub completed_count: i64,
}

#[derive(Deserialize)]
pub(crate) struct TaskInput {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub completion_mode: String,
    pub due_date: Option<String>,
    pub lesson_id: Option<String>,
}

#[derive(Serialize, FromRow)]
pub(crate) struct TaskPupil {
    pub student_id: String,
    pub name: String,
    pub completed_at: Option<i64>,
}
