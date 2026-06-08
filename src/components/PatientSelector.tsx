import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface PatientSelectorProps {
  onSelectPatient: (patient: { id: string; full_name: string }) => void;
  selectedPatientId: string | null;
}

export function PatientSelector({
  onSelectPatient,
  selectedPatientId,
}: PatientSelectorProps) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<any[]>([]);

  // Buscar pacientes cada vez que el médico escribe en el input
  useEffect(() => {
    const delayDebounce = setTimeout(async () => {
      try {
        const list = await invoke<any[]>("search_patients", {
          queryName: query,
        });
        setResults(list);
      } catch (err) {
        console.error("Error al buscar pacientes:", err);
      }
    }, 150); // Debounce rápido para no saturar el IPC de Tauri

    return () => clearTimeout(delayDebounce);
  }, [query]);

  return (
    <div style={containerStyle}>
      <h3 style={{ margin: "0 0 10px 0", color: "#f4f4f5", fontSize: "15px" }}>
        🔍 Selector Clínico de Pacientes
      </h3>
      <input
        type="text"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Escribí el nombre del paciente a evolucionar..."
        style={inputStyle}
      />

      <div style={listContainerStyle}>
        {results.length === 0 ? (
          <div style={{ color: "#71717a", fontSize: "13px", padding: "5px" }}>
            No se encontraron pacientes descifrados.
          </div>
        ) : (
          results.map((p) => {
            const isSelected = p.id === selectedPatientId;
            return (
              <div
                key={p.id}
                onClick={() => onSelectPatient(p)}
                style={{
                  ...itemStyle,
                  background: isSelected ? "#1e3a8a" : "#09090b",
                  borderColor: isSelected ? "#3b82f6" : "#27272a",
                }}
              >
                <div
                  style={{
                    fontWeight: "600",
                    color: isSelected ? "#fff" : "#e4e4e7",
                  }}
                >
                  {p.full_name}
                </div>
                <div
                  style={{
                    fontSize: "11px",
                    color: isSelected ? "#93c5fd" : "#71717a",
                    fontFamily: "monospace",
                  }}
                >
                  ID: {p.id}
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}

const containerStyle: React.CSSProperties = {
  background: "#18181b",
  padding: "20px",
  borderRadius: "8px",
  border: "1px solid #27272a",
  marginBottom: "20px",
};
const inputStyle: React.CSSProperties = {
  width: "100%",
  padding: "10px",
  borderRadius: "6px",
  border: "1px solid #27272a",
  background: "#09090b",
  color: "#f4f4f5",
  fontSize: "14px",
  boxSizing: "border-box",
  marginBottom: "12px",
};
const listContainerStyle: React.CSSProperties = {
  maxHeight: "180px",
  overflowY: "auto",
  display: "flex",
  flexDirection: "column",
  gap: "8px",
};
const itemStyle: React.CSSProperties = {
  padding: "10px",
  borderRadius: "6px",
  border: "1px solid #27272a",
  cursor: "pointer",
  transition: "all 0.2s ease",
};
