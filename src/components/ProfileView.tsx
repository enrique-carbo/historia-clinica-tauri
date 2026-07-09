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
  });
  const [isEditing, setIsEditing] = useState(false);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (activeUser?.user_id) {
      loadProfile();
    }
  }, [activeUser]);

  const loadProfile = async () => {
    try {
      const data = await invoke<{
        full_name: string;
        license_number: string;
        specialty: string;
      }>("get_my_profile", {
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

  return (
    <div className="max-w-md mx-auto p-6 bg-zinc-900 rounded-lg border border-zinc-800">
      <h2 className="text-lg font-bold text-zinc-100 mb-4">Mi Perfil</h2>

      <div className="space-y-4">
        <div>
          <label className="text-xs text-zinc-500 uppercase">Nombre completo</label>
          {isEditing ? (
            <input
              value={profile.full_name}
              onChange={(e) => setProfile(p => ({ ...p, full_name: e.target.value }))}
              className="w-full rounded bg-zinc-950 border border-zinc-700 px-3 py-2 text-sm text-zinc-200"
            />
          ) : (
            <p className="text-sm text-zinc-300">{profile.full_name || "—"}</p>
          )}
        </div>

        <div>
          <label className="text-xs text-zinc-500 uppercase">Matrícula</label>
          {isEditing ? (
            <input
              value={profile.license_number}
              onChange={(e) => setProfile(p => ({ ...p, license_number: e.target.value }))}
              className="w-full rounded bg-zinc-950 border border-zinc-700 px-3 py-2 text-sm text-zinc-200"
            />
          ) : (
            <p className="text-sm text-zinc-300">{profile.license_number || "—"}</p>
          )}
        </div>

        <div>
          <label className="text-xs text-zinc-500 uppercase">Especialidad</label>
          {isEditing ? (
            <input
              value={profile.specialty}
              onChange={(e) => setProfile(p => ({ ...p, specialty: e.target.value }))}
              className="w-full rounded bg-zinc-950 border border-zinc-700 px-3 py-2 text-sm text-zinc-200"
            />
          ) : (
            <p className="text-sm text-zinc-300">{profile.specialty || "—"}</p>
          )}
        </div>
      </div>

      <div className="mt-6 flex gap-3">
        {isEditing ? (
          <>
            <button
              onClick={handleSave}
              disabled={loading}
              className="flex-1 rounded bg-emerald-600 px-4 py-2 text-sm text-white hover:bg-emerald-700 disabled:opacity-50"
            >
              {loading ? "Guardando..." : "Guardar"}
            </button>
            <button
              onClick={() => setIsEditing(false)}
              className="flex-1 rounded bg-zinc-700 px-4 py-2 text-sm text-zinc-300 hover:bg-zinc-600"
            >
              Cancelar
            </button>
          </>
        ) : (
          <button
            onClick={() => setIsEditing(true)}
            className="w-full rounded bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700"
          >
            Editar perfil
          </button>
        )}
      </div>
    </div>
  );
}
