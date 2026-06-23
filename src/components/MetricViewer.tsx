import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Metric {
  id: string;
  metric_type: string;
  sub_metric: string;
  value_num: number;
  measured_at: string;
}

export function MetricViewer({ pacienteId }: { pacienteId: string }) {
  const [metrics, setMetrics] = useState<Metric[]>([]);

  useEffect(() => {
    invoke<Metric[]>("get_patient_metrics", { pacienteId })
      .then(setMetrics)
      .catch(console.error);
  }, [pacienteId]);

  if (metrics.length === 0) {
    return (
      <p className="text-xs text-zinc-600">Sin métricas registradas aún.</p>
    );
  }

  return (
    <div className="mt-5 text-xs text-zinc-500">
      <strong className="text-zinc-400">Historial de Métricas:</strong>
      <ul className="mt-2 pl-4 space-y-1.5">
        {metrics.map((m) => (
          <li key={m.id} className="flex gap-2">
            <span className="text-zinc-600">
              [{new Date(m.measured_at).toLocaleString()}]
            </span>
            <span className="uppercase">
              {m.metric_type} / {m.sub_metric}:
            </span>
            <strong className="text-blue-400">{m.value_num}</strong>
          </li>
        ))}
      </ul>
    </div>
  );
}
