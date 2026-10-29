export type Rule =
  | { op: 'all' | 'any'; rules: Rule[] }
  | { op: 'not'; rule: Rule }
  | { op: string; value: string | number };
export interface CleanPolicy {
  roots: string[];
  exclude: string[];
  rule: Rule;
  node_modules: boolean;
  docker: boolean;
  docker_volumes: boolean;
}
export interface CleanFile {
  path: string;
  bytes: number;
  mime: string;
  modified_ms: number;
  created_ms: number | null;
}
export interface CleanPlan {
  id: string;
  created_at: number;
  policy: CleanPolicy;
  files: CleanFile[];
  docker: { kind: string; id: string; label: string; bytes: number }[];
  total_bytes: number;
  warnings: string[];
  truncated: boolean;
}
export interface CleanOutcome {
  deleted: number;
  skipped: number;
  deleted_bytes: number;
  errors: string[];
}
export interface CleanSchedule {
  id: string;
  policy: CleanPolicy;
  interval_hours: number;
  max_bytes: number;
  next_run: number;
  last_result: CleanOutcome | null;
  enabled: boolean;
}
