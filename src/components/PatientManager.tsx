import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface PatientManagerProps {
  userId: string;
}

export function PatientManager({ userId }: PatientManagerProps) {
  const [fullName, setFullName] = useState("");
  const [identityDoc, setIdentityDoc] = useState("");
  const [patients, setPatients] = useState<any[]>([]);
  const [error, setError] = useState<string | null>(null);

  const fetchPatients = async () => {
    try {
      const res = await invoke<any[]>("get_patients_list");
      setPatients(res);
    } catch (err) {
      console.error(err);
    }
  };

  useEffect(() => {
    fetchPatients();
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    try {
      await invoke("create_patient", {
        form: {
          created_by_user_id: userId,
          full_name: fullName,
          identity_doc: identityDoc,
        },
      });
      setFullName("");
      setIdentityDoc("");
      fetchPatients();
    } catch (err) {
      setError(String(err));
    }
  };

  return (
    <div
      style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "40px" }}
    >
      {/* Formulario */}
      <div style={panelStyle}>
        <h3 style={{ margin: "0 0 15px 0", color: "#f4f4f5" }}>
          ➕ Dar de Alta Paciente
        </h3>
        {error && <div style={errorStyle}>{error}</div>}
        <form
          onSubmit={handleSubmit}
          style={{ display: "flex", flexDirection: "column", gap: "12px" }}
        >
          <div>
            <label style={labelStyle}>Nombre Completo</label>
            <input
              type="text"
              value={fullName}
              onChange={(e) => setFullName(e.target.value)}
              required
              style={inputStyle}
              placeholder="ej: Juan Pérez"
            />
          </div>
          <div>
            <label style={labelStyle}>
              Documento de Identidad (DNI / Pasaporte)
            </label>
            <input
              type="text"
              value={identityDoc}
              onChange={(e) => setIdentityDoc(e.target.value)}
              required
              style={inputStyle}
              placeholder="ej: 95123456"
            />
          </div>
          <button type="submit" style={buttonStyle}>
            Cifrar e Ingresar
          </button>
        </form>
      </div>

      {/* Lista */}
      <div style={panelStyle}>
        <h3 style={{ margin: "0 0 15px 0", color: "#f4f4f5" }}>
          📇 Padón de Pacientes Registrados
        </h3>
        <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
          {patients.length === 0 ? (
            <p style={{ color: "#71717a", fontSize: "14px" }}>
              No hay pacientes en el sistema.
            </p>
          ) : (
            patients.map((p) => (
              <div key={p.id} style={itemStyle}>
                <div>
                  <strong>{p.full_name}</strong>
                </div>
                <div
                  style={{
                    fontSize: "11px",
                    color: "#71717a",
                    fontFamily: "monospace",
                    marginTop: "4px",
                  }}
                >
                  UUID: {p.id}
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}

const panelStyle: React.CSSProperties = {
  background: "#18181b",
  padding: "20px",
  borderRadius: "8px",
  border: "1px solid #27272a",
};
const labelStyle: React.CSSProperties = {
  display: "block",
  fontSize: "12px",
  color: "#a1a1aa",
  marginBottom: "4px",
};
const inputStyle: React.CSSProperties = {
  width: "100%",
  padding: "10px",
  borderRadius: "6px",
  border: "1px solid #27272a",
  background: "#09090b",
  color: "#f4f4f5",
  boxSizing: "border-box",
};
const buttonStyle: React.CSSProperties = {
  padding: "10px",
  background: "#2563eb",
  color: "#fff",
  border: "none",
  borderRadius: "6px",
  fontWeight: "600",
  cursor: "pointer",
  marginTop: "10px",
};
const itemStyle: React.CSSProperties = {
  background: "#09090b",
  padding: "12px",
  borderRadius: "6px",
  border: "1px solid #27272a",
};
const errorStyle: React.CSSProperties = {
  background: "#450a0a",
  border: "1px solid #991b1b",
  color: "#f87171",
  padding: "10px",
  borderRadius: "6px",
  fontSize: "13px",
  marginBottom: "12px",
};
