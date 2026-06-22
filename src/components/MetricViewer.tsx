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

  if (metrics.length === 0)
    return (
      <p style={{ color: "#71717a", fontSize: "13px" }}>
        Sin métricas registradas aún.
      </p>
    );

  return (
    <div style={{ marginTop: "20px", fontSize: "13px", color: "#a1a1aa" }}>
      <strong>Historial de Métricas (Crudo):</strong>
      <ul style={{ marginTop: "10px", paddingLeft: "20px", lineHeight: "1.6" }}>
        {metrics.map((m) => (
          <li key={m.id}>
            [{new Date(m.measured_at).toLocaleString()}] - {m.metric_type} /{" "}
            {m.sub_metric}:{" "}
            <strong style={{ color: "#3b82f6" }}>{m.value_num}</strong>
          </li>
        ))}
      </ul>
    </div>
  );
}
