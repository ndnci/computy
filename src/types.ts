export type Status = 'passed' | 'warning' | 'failed' | 'unavailable' | 'cancelled';
export interface Check {
  id: string;
  component: string;
  name: string;
  status: Status;
  message: string;
  duration_ms: number;
  metrics: Record<string, any>;
  limitations: string[];
}
export interface Summary {
  status: Status;
  score: number | null;
  coverage_percent: number;
  checked: number;
  total: number;
}
export interface MemoryModule {
  locator: string;
  capacity_bytes: number;
  kind: string;
  form_factor: string;
  manufacturer: string;
  part_number: string;
  speed_mts: number | null;
  configured_speed_mts: number | null;
  replaceable: boolean | null;
}
export interface Inventory {
  runtime?: string;
  captured_at: number;
  machine_id: string;
  host_name: string;
  model: string;
  os: string;
  os_version: string;
  arch: string;
  kernel: string;
  uptime_seconds: number;
  memory: {
    total_bytes: number;
    available_bytes: number;
    test_budget_bytes?: number;
    max_capacity_bytes: number | null;
    max_supported_speed_mts: number | null;
    slots: number | null;
    modules: MemoryModule[];
    source: string;
    notes: string[];
    ecc_corrected: number | null;
    ecc_uncorrected: number | null;
  };
  cpu: {
    name: string;
    vendor: string;
    physical_cores: number | null;
    logical_cores: number;
    frequency_mhz: number;
    usage_percent: number | null;
  };
  disks: {
    name: string;
    mount_point: string;
    file_system: string;
    kind: string;
    total_bytes: number;
    available_bytes: number;
    removable: boolean;
  }[];
  temperatures: { name: string; celsius: number | null; critical_celsius: number | null }[];
  smart_available: boolean;
  notes: string[];
}
export interface RunOptions {
  components: string[];
  memory_mib: number;
  passes: number;
  disk_mib: number;
  cpu_seconds: number;
  label: string;
}
export interface Report {
  source?: string;
  schema_version: number;
  id: string;
  label: string;
  created_at: number;
  duration_ms: number;
  completed: boolean;
  options: RunOptions;
  inventory: Inventory;
  checks: Check[];
  summary: Summary;
}
export interface Job {
  id: string;
  running: boolean;
  progress: number;
  phase: string;
  component: string;
  checks: Check[];
  report: Report | null;
  error: string | null;
  cancel_requested: boolean;
}
export type Page =
  | 'overview'
  | 'memory'
  | 'storage'
  | 'cpu'
  | 'diagnostics'
  | 'history'
  | 'compare'
  | 'processes'
  | 'settings'
  | 'about';
export interface Preferences {
  memory_mib: number;
  passes: number;
  disk_mib: number;
  cpu_seconds: number;
  theme: 'system' | 'light' | 'dark';
  language: string;
  tray_enabled: boolean;
  tray_components: string[];
  tray_disks: string[];
}
export interface MonitorSnapshot {
  captured_at: number;
  cpu_percent: number | null;
  memory_percent: number | null;
  gpu_percent: number | null;
  gpu_source: string;
  processes: ProcessSample[];
  disks: {
    id: string;
    name: string;
    used_percent: number | null;
    read_bytes_per_second: number | null;
    written_bytes_per_second: number | null;
  }[];
}
export interface ProcessSample {
  pid: number;
  name: string;
  memory_bytes: number | null;
  memory_percent: number | null;
  cpu_percent: number | null;
  read_bytes_per_second: number | null;
  written_bytes_per_second: number | null;
  io_percent: number | null;
  gpu_memory_bytes: number | null;
}
declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}
