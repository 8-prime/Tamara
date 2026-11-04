// Response schemas mirror backend/src/models.rs, models/teaching.rs, and db/pupil_history.rs.
// Derive TypeScript types here so runtime validation and component contracts stay in sync.
import { z } from "zod";
export const TeacherSchema = z.object({
  id: z.string(),
  name: z.string(),
  email: z.string(),
  is_admin: z.boolean(),
});
export type Teacher = z.infer<typeof TeacherSchema>;
export const InvitationSchema = z.object({
  id: z.string(),
  label: z.string(),
  created_at: z.number(),
  expires_at: z.number(),
  used_at: z.number().nullable(),
  revoked: z.boolean(),
});
export type Invitation = z.infer<typeof InvitationSchema>;
export const ClassroomSchema = z.object({
  id: z.string(),
  name: z.string(),
  school_year: z.string(),
  role: z.string(),
  student_count: z.number(),
});
export type Classroom = z.infer<typeof ClassroomSchema>;
export const StudentSchema = z.object({
  id: z.string(),
  name: z.string(),
});
export type Student = z.infer<typeof StudentSchema>;
export const PeriodSchema = z.object({
  id: z.string(),
  name: z.string(),
  source_period_id: z.string().nullable(),
  created_at: z.number(),
});
export type Period = z.infer<typeof PeriodSchema>;
export const AssessmentSchema = z.object({
  student_id: z.string(),
  criterion_id: z.string(),
  value: z.string(),
  updated_by: z.string().nullable(),
  updated_at: z.number(),
  achieved_on: z.string().nullable(),
});
export type Assessment = z.infer<typeof AssessmentSchema>;
export const CollaboratorSchema = z.object({
  id: z.string(),
  name: z.string(),
  email: z.string(),
  role: z.string(),
});
export type Collaborator = z.infer<typeof CollaboratorSchema>;
export const SubjectSchema = z.object({
  id: z.string(),
  name: z.string(),
});
export type Subject = z.infer<typeof SubjectSchema>;
export const HomeworkSchema = z.object({
  id: z.string(),
  subject_id: z.string(),
  subject_name: z.string(),
  title: z.string(),
  description: z.string(),
  due_date: z.string(),
  status: z.string(),
  closed_at: z.number().nullable(),
  created_at: z.number(),
  assigned_count: z.number(),
  handed_in_count: z.number(),
  worksheet_name: z.string().nullable(),
});
export type Homework = z.infer<typeof HomeworkSchema>;
export const SubmissionSchema = z.object({
  student_id: z.string(),
  name: z.string(),
  handed_in_at: z.number().nullable(),
  quality: z.enum(["all_correct", "partially_correct", "missing", "all_incorrect"]).nullable(),
  note: z.string(),
});
export type Submission = z.infer<typeof SubmissionSchema>;
export const SubmissionChangeSchema = z.object({
  student_id: z.string(),
  handed_in: z.boolean().nullable().optional(),
  quality: z.enum(["all_correct", "partially_correct", "missing", "all_incorrect"]).nullable().optional(),
  note: z.string().nullable().optional(),
});
export type SubmissionChange = z.infer<typeof SubmissionChangeSchema>;
export const NoteSchema = z.object({
  id: z.string(),
  student_id: z.string(),
  content: z.string(),
  created_by: z.string().nullable(),
  created_at: z.number(),
  updated_by: z.string().nullable(),
  updated_at: z.number(),
  lesson_id: z.string().nullable(),
});
export type Note = z.infer<typeof NoteSchema>;
export const AttendanceSchema = z.object({
  student_id: z.string(),
  status: z.string(),
  updated_by: z.string().nullable(),
  updated_at: z.number(),
});
export type Attendance = z.infer<typeof AttendanceSchema>;
export const ConferenceSchema = z.object({
  id: z.string(),
  classroom_id: z.string().nullable(),
  date: z.string(),
  title: z.string().nullable(),
  content: z.string(),
  updated_at: z.number().optional(),
});
export type Conference = z.infer<typeof ConferenceSchema>;
export const LessonSchema = z.object({
  id: z.string(),
  timetable_entry_id: z.string().nullable(),
  teacher_id: z.string().nullable(),
  classroom_id: z.string(),
  class_name: z.string(),
  subject_id: z.string(),
  subject_name: z.string(),
  lesson_date: z.string(),
  starts_at: z.string(),
  ends_at: z.string(),
  room: z.string(),
  topic: z.string(),
  learning_goals: z.string(),
  status: z.string(),
  pupil_count: z.number(),
  attendance_count: z.number(),
  present_count: z.number(),
  performance_count: z.number(),
});
export type Lesson = z.infer<typeof LessonSchema>;
export const PerformanceSchema = z.object({
  student_id: z.string(),
  rating: z.number(),
  observation: z.string(),
});
export type Performance = z.infer<typeof PerformanceSchema>;
export const ClassTaskSchema = z.object({
  id: z.string(),
  classroom_id: z.string(),
  class_name: z.string(),
  lesson_id: z.string().nullable(),
  title: z.string(),
  description: z.string(),
  completion_mode: z.string(),
  due_date: z.string().nullable(),
  completed_at: z.number().nullable(),
  assigned_count: z.number(),
  completed_count: z.number(),
});
export type ClassTask = z.infer<typeof ClassTaskSchema>;
export const TaskPupilSchema = z.object({
  student_id: z.string(),
  name: z.string(),
  completed_at: z.number().nullable(),
});
export type TaskPupil = z.infer<typeof TaskPupilSchema>;
export const PupilPerformanceSchema = z.object({
  student_id: z.string(),
  lesson_id: z.string(),
  date: z.string(),
  time: z.string(),
  subject_id: z.string(),
  subject_name: z.string(),
  rating: z.number(),
  observation: z.string(),
});
export type PupilPerformance = z.infer<typeof PupilPerformanceSchema>;
export const PupilHomeworkSchema = z.object({
  student_id: z.string(),
  id: z.string(),
  title: z.string(),
  due_date: z.string(),
  subject_id: z.string(),
  subject_name: z.string(),
  handed_in_at: z.number().nullable(),
  quality: z.enum(["all_correct", "partially_correct", "missing", "all_incorrect"]).nullable(),
  note: z.string(),
});
export type PupilHomework = z.infer<typeof PupilHomeworkSchema>;
export const CriterionSchema = z.object({
  id: z.string(),
  domain: z.string(),
  section: z.string(),
  subtitle: z.string(),
  level: z.string(),
  label: z.string(),
  symbol: z.string(),
  band: z.string(),
  page: z.number(),
  x: z.number(),
  y: z.number(),
  page_height: z.number(),
});
export type Criterion = z.infer<typeof CriterionSchema>;
export const WorkspaceSchema = z.object({ students: StudentSchema.array(), periods: PeriodSchema.array(), period_id: z.string().nullable(), assessments: AssessmentSchema.array(), teachers: CollaboratorSchema.array(), role: z.string() });
export type Workspace = z.infer<typeof WorkspaceSchema>;
export const HistorySchema = z.object({ performances: PupilPerformanceSchema.array(), homework: PupilHomeworkSchema.array() });
export type History = z.infer<typeof HistorySchema>;
export const HomeworkDetailSchema = z.object({homework: HomeworkSchema, submissions: SubmissionSchema.array()});
export type HomeworkDetail = z.infer<typeof HomeworkDetailSchema>;
export const TaskDetailSchema = z.object({task: ClassTaskSchema, pupils: TaskPupilSchema.array()});
export const CatalogSchema = z.object({criteria: CriterionSchema.array()});
export const IdSchema = z.object({id: z.string()});
export const InvitationResultSchema = z.object({id: z.string(), token: z.string(), expires_in_days: z.number()});
export const AuthStatusSchema = z.object({setup_required: z.boolean()});
export type ErrorHandler = (error: unknown) => void;

export type LessonContext = Pick<Lesson, "classroom_id"> & Partial<Pick<Lesson, "id" | "lesson_date" | "subject_name" | "class_name">>;

export const AccountSummarySchema = TeacherSchema.extend({created_at: z.number(), owned_classrooms: z.number()});
export type AccountSummary = z.infer<typeof AccountSummarySchema>;
export const AccountDeletionSchema = z.object({id: z.string(), name: z.string(), email: z.string(), owned_classrooms: z.array(z.object({id: z.string(), name: z.string(), student_count: z.number()}))});
export type AccountDeletion = z.infer<typeof AccountDeletionSchema>;
export const PasswordResetResultSchema = z.object({token: z.string(), expires_at: z.number()});
