import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AuthBoxProps {
  onAuthSuccess: (user: {
    user_id: string;
    username: string;
    role: string;
  }) => void;
}

export function AuthBox({ onAuthSuccess }: AuthBoxProps) {
  const [isLogin, setIsLogin] = useState(true);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState("medico");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setLoading(true);

    try {
      if (isLogin) {
        // Ejecuta el comando nativo de Login
        const user = await invoke<{
          user_id: string;
          username: string;
          role: string;
        }>("login_user", {
          form: { username, password_plain: password },
        });
        onAuthSuccess(user);
      } else {
        // Ejecuta el comando nativo de Registro
        const userId = await invoke<string>("register_user", {
          form: { username, password_plain: password, role },
        });
        alert(`Usuario registrado con éxito localmente. ID: ${userId}`);
        setIsLogin(true);
        setPassword("");
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={boxStyle}>
      <h2 style={{ margin: "0 0 8px 0", color: "#f4f4f5", fontSize: "18px" }}>
        {isLogin ? "🔑 Iniciar Sesión Core" : "👤 Registrar Nuevo Usuario"}
      </h2>
      <p style={{ color: "#71717a", fontSize: "13px", margin: "0 0 20px 0" }}>
        {isLogin
          ? "Acceso local encriptado mediante Argon2id"
          : "El hash se calculará de forma aislada en la RAM"}
      </p>

      {error && <div style={errorStyle}>{error}</div>}

      <form
        onSubmit={handleSubmit}
        style={{ display: "flex", flexDirection: "column", gap: "14px" }}
      >
        <div>
          <label style={labelStyle}>Usuario</label>
          <input
            type="text"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            required
            style={inputStyle}
            placeholder="ej: enrique_dev"
          />
        </div>

        <div>
          <label style={labelStyle}>Contraseña</label>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            style={inputStyle}
            placeholder="••••••••"
          />
        </div>

        {!isLogin && (
          <div>
            <label style={labelStyle}>Rol Asignado</label>
            <select
              value={role}
              onChange={(e) => setRole(e.target.value)}
              style={selectStyle}
            >
              <option value="medico">Médico (Consultorio / SOAP)</option>
              <option value="admin">Administrador (Recepción / Altas)</option>
              <option value="paciente">
                Paciente (Auto-registro / Métricas)
              </option>
            </select>
          </div>
        )}

        <button type="submit" disabled={loading} style={buttonStyle}>
          {loading
            ? "Procesando en Rust..."
            : isLogin
              ? "Ingresar al Sistema"
              : "Crear Cuenta Local"}
        </button>
      </form>

      <button onClick={() => setIsLogin(!isLogin)} style={toggleButtonStyle}>
        {isLogin
          ? "¿No tenés usuario local? Registrarse"
          : "¿Ya tenés usuario? Iniciar Sesión"}
      </button>
    </div>
  );
}

const boxStyle: React.CSSProperties = {
  maxWidth: "400px",
  margin: "100px auto",
  background: "#18181b",
  padding: "30px",
  borderRadius: "8px",
  border: "1px solid #27272a",
  boxSizing: "border-box",
};
const labelStyle: React.CSSProperties = {
  display: "block",
  fontSize: "12px",
  color: "#a1a1aa",
  marginBottom: "4px",
  fontWeight: "600",
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
};
const selectStyle: React.CSSProperties = {
  width: "100%",
  padding: "10px",
  borderRadius: "6px",
  border: "1px solid #27272a",
  background: "#09090b",
  color: "#f4f4f5",
  fontSize: "14px",
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
const toggleButtonStyle: React.CSSProperties = {
  background: "none",
  border: "none",
  color: "#3b82f6",
  fontSize: "13px",
  cursor: "pointer",
  marginTop: "15px",
  padding: 0,
  textDecoration: "underline",
  width: "100%",
  textAlign: "center",
};
const errorStyle: React.CSSProperties = {
  background: "#450a0a",
  border: "1px solid #991b1b",
  color: "#f87171",
  padding: "10px",
  borderRadius: "6px",
  fontSize: "13px",
  marginBottom: "15px",
};
