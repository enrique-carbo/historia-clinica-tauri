import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { SoapForm } from "./components/SoapForm";
import { SoapHistory } from "./components/SoapHistory";
import { AuthBox } from "./components/AuthBox";
import { PatientManager } from "./components/PatientManager";
import { PatientSelector } from "./components/PatientSelector";

interface CurrentUser {
  user_id: string;
  username: string;
  role: string;
}

export default function App() {
  const [user, setUser] = useState<CurrentUser | null>(null);
  //const [pacienteId] = useState("paciente_123_test"); // Sigue estático hasta el siguiente módulo
  const [history, setHistory] = useState<any[]>([]);
  const [activePatient, setActivePatient] = useState<{
    id: string;
    full_name: string;
  } | null>(null);

  const fetchHistory = useCallback(async () => {
    if (!activePatient) {
      setHistory([]);
      return;
    }
    try {
      const res = await invoke<any[]>("get_patient_history", {
        pacienteId: activePatient.id,
      });
      setHistory(res);
    } catch (err) {
      console.error("Error al leer historial:", err);
    }
  }, [activePatient]);

  useEffect(() => {
    if (user && user.role === "medico") {
      fetchHistory();
    }
  }, [user, activePatient, fetchHistory]);

  // Si el usuario no está autenticado localmente, forzamos el AuthBox
  if (!user) {
    return <AuthBox onAuthSuccess={(loggedUser) => setUser(loggedUser)} />;
  }

  return (
    <div
      style={{
        padding: "40px",
        fontFamily: "system-ui, sans-serif",
        background: "#0f0f11",
        color: "#e4e4e7",
        minHeight: "100vh",
      }}
    >
      <header
        style={{
          marginBottom: "30px",
          borderBottom: "1px solid #27272a",
          paddingBottom: "20px",
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
        }}
      >
        <div>
          <h1 style={{ fontSize: "24px", color: "#f4f4f5", margin: 0 }}>
            🩺 Simplex Health Core
          </h1>
          <p style={{ color: "#a1a1aa", fontSize: "14px", marginTop: "5px" }}>
            Ecosistema Clínico Descentralizado — Rol:{" "}
            <strong style={{ color: "#2563eb", textTransform: "uppercase" }}>
              {user.role}
            </strong>
          </p>
        </div>
        <div style={{ textAlign: "right" }}>
          <span style={{ fontSize: "14px", color: "#a1a1aa" }}>
            Usuario: <strong>{user.username}</strong>
          </span>
          <button
            onClick={() => setUser(null)}
            style={{
              marginLeft: "15px",
              padding: "6px 12px",
              background: "#27272a",
              border: "1px solid #3f3f46",
              color: "#f4f4f5",
              borderRadius: "4px",
              cursor: "pointer",
              fontSize: "12px",
            }}
          >
            Cerrar Sesión
          </button>
        </div>
      </header>

      {user.role === "admin" && (
        <div>
          <h2 style={{ fontSize: "18px", marginBottom: "20px" }}>
            Consola de Recepción y Admisión
          </h2>
          <PatientManager userId={user.user_id} />
        </div>
      )}

      {user.role === "medico" && (
        <div style={{ display: "flex", flexDirection: "column", gap: "30px" }}>
          {/* Sección de Admisión de Pacientes */}
          <div>
            <h2 style={{ fontSize: "18px", marginBottom: "15px" }}>
              Admitir Nuevo Paciente
            </h2>
            <PatientManager userId={user.user_id} />
          </div>

          <hr
            style={{
              border: "none",
              borderTop: "1px solid #27272a",
              margin: "10px 0",
            }}
          />

          {/* Buscador de Pacientes Activos */}
          <PatientSelector
            onSelectPatient={(p) => setActivePatient(p)}
            selectedPatientId={activePatient ? activePatient.id : null}
          />

          {/* Panel Clínico Sincrónico Dinámico */}
          {activePatient ? (
            <div>
              <h2 style={{ fontSize: "18px", marginBottom: "15px" }}>
                Consulta Activa:{" "}
                <span style={{ color: "#3b82f6" }}>
                  {activePatient.full_name}
                </span>
              </h2>
              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: "1fr 1fr",
                  gap: "40px",
                  maxWidth: "1400px",
                }}
              >
                <div>
                  <div
                    style={{
                      marginBottom: "15px",
                      fontSize: "13px",
                      color: "#71717a",
                    }}
                  >
                    VÍNCULO RELACIONAL:{" "}
                    <span style={{ color: "#e4e4e7", fontFamily: "monospace" }}>
                      {activePatient.id}
                    </span>
                  </div>
                  <SoapForm
                    pacienteId={activePatient.id}
                    medicoId={user.user_id}
                    onSuccess={fetchHistory}
                  />
                </div>
                <div>
                  <h3
                    style={{
                      margin: "0 0 15px 0",
                      color: "#f4f4f5",
                      fontSize: "16px",
                    }}
                  >
                    Historial Clínico Cifrado
                  </h3>
                  <SoapHistory records={history} />
                </div>
              </div>
            </div>
          ) : (
            <div
              style={{
                padding: "30px",
                textAlign: "center",
                background: "#141417",
                borderRadius: "8px",
                border: "1px dashed #27272a",
                color: "#71717a",
              }}
            >
              💡 Seleccioná un paciente del buscador para abrir su ficha clínica
              y redactar una evolución SOAP.
            </div>
          )}
        </div>
      )}
    </div>
  );
}
