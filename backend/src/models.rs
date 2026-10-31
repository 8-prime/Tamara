//! Shared records and commands. Repositories never depend on API request types.
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
pub(crate) mod teaching;

#[derive(Serialize, FromRow)]
pub(crate) struct Teacher {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) is_admin: bool,
}

#[derive(Serialize, FromRow)]
pub(crate) struct Invitation {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) created_at: i64,
    pub(crate) expires_at: i64,
    pub(crate) used_at: Option<i64>,
    pub(crate) revoked: bool,
}

#[derive(Serialize, FromRow)]
pub(crate) struct Classroom {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) school_year: String,
    pub(crate) role: String,
    pub(crate) student_count: i64,
}

#[derive(Serialize, FromRow)]
pub(crate) struct Student {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Serialize, FromRow)]
pub(crate) struct Period {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) source_period_id: Option<String>,
    pub(crate) created_at: i64,
}

#[derive(Serialize, FromRow)]
pub(crate) struct Assessment {
    pub(crate) student_id: String,
    pub(crate) criterion_id: String,
    pub(crate) value: String,
    pub(crate) updated_by: String,
    pub(crate) updated_at: i64,
    pub(crate) achieved_on: Option<String>,
}

#[derive(Serialize, FromRow)]
pub(crate) struct Collaborator {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) role: String,
}

#[derive(Deserialize)]
pub(crate) struct AssessmentChange {
    pub(crate) student_id: String,
    pub(crate) criterion_id: String,
    pub(crate) value: Option<String>,
    pub(crate) achieved_on: Option<String>,
}

#[derive(Serialize, FromRow)]
pub(crate) struct Subject {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Serialize, FromRow)]
pub(crate) struct Homework {
    pub(crate) id: String,
    pub(crate) subject_id: String,
    pub(crate) subject_name: String,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) due_date: String,
    pub(crate) status: String,
    pub(crate) closed_at: Option<i64>,
    pub(crate) created_at: i64,
    pub(crate) assigned_count: i64,
    pub(crate) handed_in_count: i64,
    pub(crate) worksheet_name: Option<String>,
}

#[derive(Serialize, FromRow)]
pub(crate) struct Submission {
    pub(crate) student_id: String,
    pub(crate) name: String,
    pub(crate) handed_in_at: Option<i64>,
    pub(crate) quality: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct HomeworkInput {
    pub(crate) subject_id: String,
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) description: String,
    pub(crate) due_date: String,
}

#[derive(Deserialize)]
pub(crate) struct SubmissionChange {
    pub(crate) student_id: String,
    pub(crate) handed_in: Option<bool>,
    // Omitted preserves the quality; explicit null clears it.
    #[serde(default, deserialize_with = "submission_quality")]
    pub(crate) quality: Option<Option<String>>,
}

fn submission_quality<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(Some)
}

#[derive(Serialize, FromRow)]
pub(crate) struct Note {
    pub(crate) id: String,
    pub(crate) student_id: String,
    pub(crate) content: String,
    pub(crate) created_by: String,
    pub(crate) created_at: i64,
    pub(crate) updated_by: String,
    pub(crate) updated_at: i64,
    pub(crate) lesson_id: Option<String>,
}

#[derive(Serialize, FromRow)]
pub(crate) struct Attendance {
    pub(crate) student_id: String,
    pub(crate) status: String,
    pub(crate) updated_by: String,
    pub(crate) updated_at: i64,
}

#[derive(Deserialize)]
pub(crate) struct AttendanceChange {
    pub(crate) student_id: String,
    pub(crate) status: Option<String>,
}

// Password hashes stay out of serializable teacher responses.
#[derive(FromRow)]
pub(crate) struct Account {
    #[sqlx(flatten)]
    pub(crate) teacher: Teacher,
    pub(crate) password_hash: String,
}
