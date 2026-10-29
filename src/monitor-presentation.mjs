export function usageColor(value) {
  return value === null || !Number.isFinite(value)
    ? '#8b9b9d'
    : value >= 90
      ? '#e96565'
      : value >= 65
        ? '#dca842'
        : '#429feb';
}
export const metricNames = {
  memory: 'Mémoire vive',
  cpu: 'Processeur',
  gpu: 'Processeur graphique',
  storage: 'Stockage',
};
export function metricValue(snapshot, metric, volume = '') {
  if (!snapshot) return null;
  return metric === 'storage'
    ? (snapshot.disks.find((d) => d.id === volume)?.used_percent ?? null)
    : (snapshot[`${metric}_percent`] ?? null);
}
export function processValue(process, metric) {
  return metric === 'memory'
    ? process.memory_bytes
    : metric === 'cpu'
      ? process.cpu_percent
      : metric === 'gpu'
        ? process.gpu_memory_bytes
        : process.read_bytes_per_second === null || process.written_bytes_per_second === null
          ? null
          : process.read_bytes_per_second + process.written_bytes_per_second;
}
export function rankedProcesses(processes, metric) {
  return [...processes].sort(
    (a, b) => (processValue(b, metric) ?? -1) - (processValue(a, metric) ?? -1) || a.pid - b.pid,
  );
}
