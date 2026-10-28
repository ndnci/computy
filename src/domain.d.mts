import type { Check, Report, Status } from './types';
export function formatBytes(value: number | null | undefined, digits?: number): string;
export function formatDate(value: number, compact?: boolean): string;
export function duration(ms: number): string;
export function statusInfo(status: Status | string): { label: string; short: string; tone: string };
export function componentSummary(
  checks: Check[],
  component: string,
): { score: number | null; status: Status; checked: number; total: number };
export function comparisonWarnings(reports: Report[]): string[];
export function scoreStatus(score: number | null, status: Status): Status;
export function checkOutcome(
  checks: Pick<Check, 'status' | 'name'>[],
  completed?: boolean,
): { status: Status; unavailable: string[] };
