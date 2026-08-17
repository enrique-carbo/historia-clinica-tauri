// src/components/ProfileView.tsx

import { useState, useEffect } from "react";
import { useAuthStore } from "../stores/useAuthStore";
import { invoke } from "@tauri-apps/api/core";

export function ProfileView() {
  const { activeUser } = useAuthStore();
  const [profile, setProfile] = useState({
    full_name: "",
    license_number: "",
    specialty: "",
    address: "",
    phone: "",
    email: "",
    website: "",
  });
  const [isEditing, setIsEditing] = useState(false);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (activeUser?.user_id) loadProfile();
  }, [activeUser]);

  const loadProfile = async () => {
    try {
      const data = await invoke<any>("get_my_profile", {
        userId: activeUser!.user_id,
      });
      setProfile(data);
    } catch (e) {
      console.error("Error cargando perfil:", e);
    }
  };

  const handleSave = async () => {
    if (!activeUser) return;
    setLoading(true);
    try {
      await invoke("update_my_profile", {
        input: {
          user_id: activeUser.user_id,
          ...profile,
        },
      });
      setIsEditing(false);
    } catch (e) {
      console.error("Error guardando perfil:", e);
    } finally {
      setLoading(false);
    }
  };

  const handleChange = (field: string, value: string) => {
    setProfile((p) => ({ ...p, [field]: value }));
  };

  return (
    <div className="max-w-4xl mx-auto p-8 bg-zinc-900 rounded-xl border border-zinc-800 shadow-lg">
      <div className="flex justify-between items-center mb-8 border-b border-zinc-800 pb-4">
        <h2 className="text-2xl font-bold text-zinc-100 flex items-center gap-2">
          👤 Perfil Profesional
        </h2>
        {!isEditing && (
          <button
            onClick={() => setIsEditing(true)}
            className="rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 transition-colors"
          >
            Editar Información
          </button>
        )}
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
        {/* Columna Izquierda: Datos Clínicos */}
        <div className="space-y-5">
          <h3 className="text-xs font-bold text-blue-400 uppercase tracking-widest mb-4">
            Identidad Clínica
          </h3>

          {[
            { label: "Nombre Completo", field: "full_name" },
            { label: "Matrícula Profesional", field: "license_number" },
            { label: "Especialidad", field: "specialty" },
          ].map((item) => (
            <div key={item.field}>
              <label className="block text-xs font-semibold text-zinc-500 mb-1.5 uppercase">
                {item.label}
              </label>
              {isEditing ? (
                <input
                  value={(profile as any)[item.field]}
                  onChange={(e) => handleChange(item.field, e.target.value)}
                  className="w-full rounded-lg bg-zinc-950 border border-zinc-700 px-3 py-2.5 text-sm text-zinc-200 focus:border-blue-500 outline-none transition-all"
                />
              ) : (
                <p className="text-sm text-zinc-300 font-medium py-2.5 border-b border-zinc-800/50">
                  {(profile as any)[item.field] || "—"}
                </p>
              )}
            </div>
          ))}
        </div>

        {/* Columna Derecha: Contacto Profesional */}
        <div className="space-y-5">
          <h3 className="text-xs font-bold text-emerald-400 uppercase tracking-widest mb-4">
            Datos de Contacto
          </h3>

          {[
            { label: "Domicilio Profesional", field: "address", type: "text" },
            { label: "Teléfono de Consultorio", field: "phone", type: "tel" },
            { label: "Correo Electrónico", field: "email", type: "email" },
            { label: "Sitio Web / Agenda", field: "website", type: "url" },
          ].map((item) => (
            <div key={item.field}>
              <label className="block text-xs font-semibold text-zinc-500 mb-1.5 uppercase">
                {item.label}
              </label>
              {isEditing ? (
                <input
                  type={item.type}
                  value={(profile as any)[item.field]}
                  onChange={(e) => handleChange(item.field, e.target.value)}
                  className="w-full rounded-lg bg-zinc-950 border border-zinc-700 px-3 py-2.5 text-sm text-zinc-200 focus:border-emerald-500 outline-none transition-all"
                />
              ) : (
                <p className="text-sm text-zinc-300 font-medium py-2.5 border-b border-zinc-800/50 truncate">
                  {(profile as any)[item.field] || "—"}
                </p>
              )}
            </div>
          ))}
        </div>
      </div>

      {isEditing && (
        <div className="mt-8 pt-6 border-t border-zinc-800 flex justify-end gap-3">
          <button
            onClick={() => setIsEditing(false)}
            className="rounded-lg bg-zinc-800 px-6 py-2.5 text-sm font-bold text-zinc-300 hover:bg-zinc-700 transition-all"
          >
            Cancelar
          </button>
          <button
            onClick={handleSave}
            disabled={loading}
            className="rounded-lg bg-emerald-600 px-6 py-2.5 text-sm font-bold text-white hover:bg-emerald-700 disabled:opacity-50 shadow-lg shadow-emerald-900/20 transition-all"
          >
            {loading ? "Guardando cambios..." : "Guardar Perfil"}
          </button>
        </div>
      )}
    </div>
  );
}
