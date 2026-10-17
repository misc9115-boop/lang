import { invoke, isTauri } from "@tauri-apps/api/core";

export type Settings = {
  level: string;
  timeZone: string;
  dailyGoal: number;
  theme: string;
  reducedMotion: boolean;
  textScale: number;
};
export type Answer =
  | { type: "text"; value: string }
  | { type: "option"; value: string }
  | { type: "tiles"; value: string[] };
export type Lesson = {
  id: string;
  level: string;
  title: string;
  objective: string;
  editorialStatus: string;
  exerciseCount: number;
};
export type Exercise = {
  id: string;
  kind: "choice" | "translation" | "ordering" | "cloze" | "reading";
  instruction: string;
  prompt: string;
  context: string | null;
  options: { id: string; text: string }[];
  hint: string | null;
  draft: Answer | null;
  feedback: {
    correct: boolean;
    revealed: boolean;
    expected: string;
    explanation: string;
  } | null;
  readyToContinue: boolean;
};
export type Session = {
  id: string;
  lesson: Lesson;
  position: number;
  status: string;
  exercise: Exercise | null;
  firstCorrect: number;
  independentCorrect: number;
  hintedItems: number;
  revealedItems: number;
  xpEarned: number;
  practiceOnly: boolean;
};
export type Progress = {
  totalXp: number;
  todayXp: number;
  completedLessons: number;
  currentStreak: number;
  bestStreak: number;
  achievements: string[];
  days: { date: string; xp: number; goal: number; timeZone: string }[];
};
export type AppState = {
  settings: Settings | null;
  lessons: Lesson[];
  activeSession: Session | null;
  progress: Progress;
  developmentBuild: boolean;
  contentIssues: string[];
};
export type BackupTicket = {
  token: string;
  preview: {
    createdAt: string;
    level: string | null;
    totalXp: number;
    completedLessons: number;
    sessionCount: number;
    activeLesson: string | null;
    development: boolean;
    fingerprint: string;
    packReferences: PackReference[];
    reviewItemCount: number;
  };
};
export type RecoveryEntry = { id: string; savedAt: string };
export type PackReference = {
  packId: string;
  version: number;
  manifestSha256: string;
};
export type PackRecord = {
  entry: PackReference & {
    stored: boolean;
    active: boolean;
    payloadBytes: number;
    installedAt: string;
  };
  title: string | null;
  editorialStatus: "draft" | "approved" | null;
  error: string | null;
};
export type PackState = { entries: PackRecord[]; issues: string[] };
export type PackTicket = {
  token: string;
  packId: string;
  version: number;
  title: string;
  payloadBytes: number;
  lessonCount: number;
  editorialStatus: "draft" | "approved";
  license: string;
  attribution: string;
};

export async function command<T>(
  name: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauri())
    throw new Error(
      "Open the desktop app with npm run tauri dev. This browser view has no local database connection.",
    );
  return invoke<T>(name, args);
}

export const levels = ["Beginner", "A1", "A2", "B1", "B2", "C1", "C2"];
// Summaries of the approved map in SPEC.md §5. These describe planned coverage,
// not installed, reviewed content or a certification claim.
export const coverage: Record<string, { title: string; description: string }> =
  {
    Beginner: {
      title: "Start with the everyday",
      description:
        "Begin with A1 foundations: Italian sounds, greetings, introductions, and simple requests.",
    },
    A1: {
      title: "Make yourself understood",
      description:
        "Everyday expressions, introductions, family, food, and travel basics. Articles, agreement, and the present tense.",
    },
    A2: {
      title: "Find your daily rhythm",
      description:
        "Everyday transactions, home and work, directions, routines, and short narratives.",
    },
    B1: {
      title: "Keep the conversation going",
      description:
        "Experiences, plans, opinions, narration, and everyday problem-solving.",
    },
    B2: {
      title: "Express your point of view",
      description:
        "Arguments, work and study, complex sentences, register, and sustained comprehension.",
    },
    C1: {
      title: "Read between the lines",
      description:
        "Demanding texts, implicit meaning, persuasive structure, and professional or academic expression.",
    },
    C2: {
      title: "Find the precise expression",
      description:
        "Subtle style, dense or idiomatic language, synthesis across sources, and nuanced argument.",
    },
  };
