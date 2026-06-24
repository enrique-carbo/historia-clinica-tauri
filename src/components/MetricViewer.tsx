import { usePatientStore } from "../stores/usePatientStore";

export function MetricViewer() {
  // 🚀 Se suscribe al estado global. Se re-renderiza solo si cambian las métricas.
  const metrics = usePatientStore((state) => state.metrics);

  if (metrics.length === 0) {
    return (
      <p className="text-xs text-zinc-600 mt-5">
        Sin métricas registradas aún.
      </p>
    );
  }

  return (
    <div className="mt-5 text-xs text-zinc-500 border-t border-zinc-800 pt-4">
      <strong className="text-zinc-400">
        Historial de Métricas Desidentificado:
      </strong>
      <ul className="mt-2 pl-4 space-y-1.5">
        {metrics.map((m) => (
          <li key={m.id} className="flex gap-2 items-baseline">
            <span className="text-zinc-600 font-mono">
              {new Date(m.measured_at).toLocaleString()}
            </span>
            <span className="uppercase">
              {m.metric_type} / {m.sub_metric}:
            </span>
            {/* 🚀 Formateo seguro para evitar floating point feos (ej: 80.5 en lugar de 80.50000001) */}
            <strong className="text-blue-400">
              {Number.isInteger(m.value_num)
                ? m.value_num
                : m.value_num.toFixed(1)}
            </strong>
          </li>
        ))}
      </ul>
    </div>
  );
}
